use crate::glue;
use crate::job::{self, Job};
use crate::store::Store;
use crate::workspace::Workspace;
use anyhow::{Context, Result};
use core::convert::{TryFrom, TryInto};
use std::collections::{HashMap, HashSet};
use std::fs::{File, Metadata};
use std::hash::{Hash, Hasher};
use std::io::BufReader;
use std::path::PathBuf;
use std::time::SystemTime;
use xxhash_rust::xxh3::Xxh3;

#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;

pub struct Builder {
    workspace_root: PathBuf,
    store: Store,
    targets: Vec<glue::Job>,
}

impl Builder {
    pub fn new(workspace_root: PathBuf, store: Store) -> Self {
        Builder {
            workspace_root,
            store,

            // it's very likely we'll have at least one target
            targets: Vec::with_capacity(1),
        }
    }

    pub fn add_target(&mut self, job: glue::Job) {
        self.targets.push(job);
    }

    pub fn build(mut self) -> Result<Coordinator> {
        // Here's the overview of what we're about to do: for each file in
        // each target job, we're going to look at metadata for that file and
        // use that metadata to look up the file's hash (if we don't have it
        // already, we'll read the file and calculate it.) We'll use all those
        // hashes to construct a mapping from path->hash that coordinator can
        // use to determine which jobs need to be run or skipped once the build
        // is running.
        //
        // For more higher-level explanation of what we're going for, refer
        // to docs/internals/how-we-determine-when-to-run-jobs.md.

        // We're currently storing the mapping from PathMetaKey to content hash as
        // a JSON object, so we need to load it before we can do anything else. In
        // the longer term, we'll probably move to using some sort of KV store,
        // at which point this deserialization will just be opening the database.
        let file_hashes_path = self.workspace_root.join("file_hashes.json");
        let mut meta_to_hash: HashMap<u64, String> = match File::open(&file_hashes_path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                serde_json::from_reader(reader)
                    .context("could not deserialize mapping from inputs to content")?
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => HashMap::default(),
            Err(err) => return Err(err).context("could not open file hash cache"),
        };

        // We assume that there will be at least some overlap in inputs (i.e. at
        // least two targets needing the same file.) That assumption means that
        // it makes sense to deduplicate them to avoid duplicating filesystem
        // operations.
        let mut input_files: HashSet<PathBuf> = HashSet::new();
        for glue_job in &self.targets {
            for file in &glue_job.as_Job().inputFiles {
                input_files.insert(
                    job::sanitize_file_path(file).context("got an unacceptable input file path")?,
                );
            }
        }

        let mut coordinator = Coordinator {
            workspace_root: self.workspace_root,
            store: self.store,

            path_to_hash: HashMap::with_capacity(input_files.len()),

            // On capacities: we'll have at least as many jobs as we have targets,
            // each of which will have at least one leaf node.
            jobs: HashMap::with_capacity(self.targets.len()),
            blocked: HashMap::default(),
            ready: Vec::with_capacity(self.targets.len()),
        };

        /////////////////////////////////////////////
        // Phase 1: check which files have changed //
        /////////////////////////////////////////////

        let mut path_to_meta: HashMap<PathBuf, PathMetaKey> =
            HashMap::with_capacity(input_files.len());

        // TODO: perf hint for later: we could be doing this in parallel
        // using rayon
        for input_file in input_files {
            // TODO: collect errors instead of bailing immediately
            let meta = input_file.metadata().with_context(|| {
                format!("could not read metadata for `{}`", input_file.display())
            })?;

            if meta.is_dir() {
                anyhow::bail!(
                    "One of your jobs specifies `{}` as a dependency. It's a directory, but I can only handle files.",
                    input_file.display(),
                )
            };

            let cache_key = meta.try_into().with_context(|| {
                format!(
                    "could not calculate a cache key for `{}`",
                    input_file.display()
                )
            })?;

            path_to_meta.insert(input_file, cache_key);
        }

        ///////////////////////////////////////////////////////////////////
        // Phase 2a: get hashes for metadata keys we haven't seen before //
        ///////////////////////////////////////////////////////////////////
        let mut hasher = blake3::Hasher::new();

        for (path, cache_key) in path_to_meta.iter() {
            let key = u64::from(cache_key);
            if let Some(hash) = meta_to_hash.get(&key) {
                coordinator
                    .path_to_hash
                    .insert(path.to_path_buf(), hash.to_owned());
                continue;
            }

            let mut file = File::open(&path)
                .with_context(|| format!("couldn't open `{}` for hashing.", path.display()))?;

            hasher.reset();

            // TODO: docs for Blake3 say that a 16 KiB buffer is the most
            // efficient (for SIMD reasons), but `std::io::copy` uses an 8KiB
            // buffer. Gonna have to do this by hand at some point to take
            // advantage of the algorithm's designed speed.
            std::io::copy(&mut file, &mut hasher)?;

            let hash = hasher.finalize();

            log::debug!("hash of `{}` was {}", path.display(), hash);
            meta_to_hash.insert(key, hash.to_string());

            coordinator
                .path_to_hash
                .insert(path.to_path_buf(), hash.to_string());
        }

        ////////////////////////////////////////////////////////////////
        // Phase 2b: keep track of the hashes to avoid work next time //
        ////////////////////////////////////////////////////////////////
        let file_hashes = File::create(file_hashes_path)
            .context("could not open file hash cache to store new hashes")?;
        // TODO: BufWriter?
        serde_json::to_writer(file_hashes, &meta_to_hash)
            .context("failed to write hash cache to disk")?;

        ///////////////////////////////////////////////////////////////////////////
        // Phase 3: get the hahes to determine what jobs we actually need to run //
        ///////////////////////////////////////////////////////////////////////////
        for glue_job in self.targets.drain(..) {
            // Note: this data structure is going to grow the ability to
            // refer to other jobs as soon as it's possibly feasible. When
            // that happens, a depth-first search through the tree rooted at
            // `glue_job` will probably suffice.
            let job =
                Job::from_glue(glue_job).context("could not convert glue job to actual job")?;

            // TODO: pushing to `ready` immediately is only reasonable when
            // we don't have job inputs as dependencies, but we don't have that
            // yet. This'll need to change when we do or we'll have some very
            // broken runs!
            coordinator.ready.push(job.base_key);
            coordinator.jobs.insert(job.base_key, job);
        }

        Ok(coordinator)
    }
}

#[derive(Debug)]
pub struct Coordinator {
    workspace_root: PathBuf,
    store: Store,

    // caches
    path_to_hash: HashMap<PathBuf, String>,

    // which jobs should run when?
    jobs: HashMap<job::Key, Job>,
    blocked: HashMap<job::Key, HashSet<job::Key>>,
    ready: Vec<job::Key>,
}

impl Coordinator {
    pub fn has_outstanding_work(&self) -> bool {
        !self.blocked.is_empty() || !self.ready.is_empty()
    }

    pub fn run_next<R: Runner>(&mut self, runner: &R) -> Result<()> {
        let id = match self.ready.pop() {
            Some(id) => id,
            None => anyhow::bail!("no work ready to do"),
        };

        let job = self
            .jobs
            .get(&id)
            .context("had a bad job ID in Coordinator.ready")?;

        log::debug!("preparing to run job {}", job);

        // figure out the final key based on the job's dependencies
        let mut key_builder = job::KeyBuilder::based_on(&job.base_key);
        for path in &job.input_files {
            match self.path_to_hash.get(path) {
                Some(hash) => key_builder.add_file(&path, hash),
                None => anyhow::bail!("`{}` was specified as a file dependency, but I didn't have a hash for it! This is a bug in rbt's coordinator, please file it!", path.display()),
            }
        }
        let key = key_builder.finalize();

        // build (or don't) based on the final key!
        if self.store.for_job(&key).is_none() {
            let workspace = Workspace::create(&self.workspace_root, &key)
                .with_context(|| format!("could not create workspace for {}", job))?;

            runner.run(job, &workspace).context("could not run job")?;

            self.store
                .store_from_workspace(key, &job, workspace)
                .context("could not store job output")?;
        } else {
            log::debug!("already had output of job {}; skipping", job);
        }

        // Now that we're done running the job, we update our bookkeeping to
        // figure out what running that job just unblocked.
        //
        // As an implementation note, this will probably end up in a separate
        // function once we're running tasks in parallel!
        let mut newly_unblocked = vec![]; // avoiding mutating both fields of self in the loop below

        self.blocked.retain(|blocked, blockers| {
            let removed = blockers.remove(&id);
            if !removed {
                return false;
            }

            let no_blockers_remaining = blockers.is_empty();
            if no_blockers_remaining {
                log::debug!("unblocked {}", blocked);
                newly_unblocked.push(*blocked)
            }
            !no_blockers_remaining
        });
        self.ready.extend(newly_unblocked);

        Ok(())
    }
}

pub trait Runner {
    fn run(&self, job: &Job, workspace: &Workspace) -> Result<()>;
}

impl Runner for Box<dyn Runner> {
    fn run(&self, job: &Job, workspace: &Workspace) -> Result<()> {
        self.as_ref().run(job, workspace)
    }
}

#[derive(Debug, Hash)]
struct PathMetaKey {
    // common
    modified: SystemTime,
    len: u64,

    // Unix-only
    #[cfg(target_family = "unix")]
    inode: u64,
    #[cfg(target_family = "unix")]
    mode: u32,
    #[cfg(target_family = "unix")]
    uid: u32,
    #[cfg(target_family = "unix")]
    gid: u32,
    // TODO: extra info for Windows
}

impl From<&PathMetaKey> for u64 {
    fn from(key: &PathMetaKey) -> Self {
        let mut hasher = Xxh3::new();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(target_family = "unix")]
impl TryFrom<Metadata> for PathMetaKey {
    type Error = anyhow::Error;

    fn try_from(meta: Metadata) -> Result<PathMetaKey> {
        Ok(PathMetaKey {
            modified: meta
                .modified()
                .context("mtime is not supported on this system")?,
            len: meta.len(),
            inode: meta.ino(),
            mode: meta.mode(),
            uid: meta.uid(),
            gid: meta.gid(),
        })
    }
}

#[cfg(not(target_family = "unix"))]
impl TryFrom<Metadata> for PathMetaKey {
    type Error = anyhow::Error;

    fn try_from(meta: Metadata) -> Result<PathMetaKey> {
        Ok(PathMetaKey {
            modified: meta
                .modified()
                .context("mtime is not supported on this system")?,
            len: meta.len(),
        })
    }
}
