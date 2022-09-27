use crate::glue;
use crate::job::{self, Job};
use crate::store::{self, Store};
use crate::workspace::Workspace;
use anyhow::{Context, Result};
use core::convert::{TryFrom, TryInto};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::fs::{File, Metadata};
use std::hash::{Hash, Hasher};
use std::io::BufReader;
use std::path::PathBuf;
use std::time::SystemTime;
use xxhash_rust::xxh3::{Xxh3, Xxh3Builder};

#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;

pub struct Builder<'roc> {
    workspace_root: PathBuf,
    store: Store,
    roots: Vec<&'roc glue::Job>,
}

impl<'roc> Builder<'roc> {
    pub fn new(workspace_root: PathBuf, store: Store) -> Self {
        Builder {
            workspace_root,
            store,

            // it's very likely we'll have at least one target
            roots: Vec::with_capacity(1),
        }
    }

    pub fn add_target(&mut self, job: &'roc glue::Job) {
        self.roots.push(job);
    }

    pub fn build(self) -> Result<Coordinator<'roc>> {
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
        for glue_job in &self.roots {
            for input in &glue_job.as_Job().inputs {
                if input.discriminant() == glue::discriminant_U1::FromProjectSource {
                    for file in unsafe { input.as_FromProjectSource() } {
                        input_files.insert(job::sanitize_file_path(file)?);
                    }
                }
            }
        }

        let mut coordinator = Coordinator {
            workspace_root: self.workspace_root,
            store: self.store,

            path_to_hash: HashMap::with_capacity(input_files.len()),
            job_to_content_hash: HashMap::with_capacity(self.roots.len()),

            // On capacities: we'll have at least as many jobs as we have targets,
            // each of which will have at least one leaf node.
            jobs: HashMap::with_capacity(self.roots.len()),
            blocked: HashMap::default(),
            ready: Vec::with_capacity(self.roots.len()),
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

        // to build a graph, we need the base keys for all jobs. This can't be
        // a depth-first search, however, because that would mean processing
        // dependent jobs before their dependencies. We can't do that because
        // then we would have unresolved base keys and we'd have to defer until
        // we got them or temporarily suffer incomplete information in the
        // graph.
        //
        // Ideally, we'd look at the leaf nodes first, then the things that
        // depend on them, etc. In other words, a depth-first search starting
        // at the leaves instead of the roots. Lucky for us, that's easy to do:
        // just write down the jobs we see as we do a depth first search, then
        // walk that list in the opposite direction.
        //
        // `to_descend_into` tracks the depth-first search part of this scheme,
        // and `to_convert` is where we write down the dependencies in root-to-
        // leaf order.
        let mut to_descend_into = self.roots.clone();
        let mut to_convert = Vec::with_capacity(self.roots.len());

        let mut glue_to_job_key: HashMap<&glue::Job, job::Key<job::Base>, Xxh3Builder> =
            HashMap::with_capacity_and_hasher(self.roots.len(), Xxh3Builder::new());

        let mut job_deps: HashMap<&glue::Job, HashSet<&glue::Job, Xxh3Builder>, Xxh3Builder> =
            HashMap::with_hasher(Xxh3Builder::new());

        while let Some(next_glue_job) = to_descend_into.pop() {
            next_glue_job
                .as_Job()
                .inputs
                .iter()
                .filter(|item| item.discriminant() == glue::discriminant_U1::FromJob)
                .for_each(|item| {
                    let job = unsafe { item.as_FromJob() }.0;

                    let entry = job_deps.entry(next_glue_job);
                    entry
                        .or_insert_with(|| HashSet::with_capacity_and_hasher(1, Xxh3Builder::new()))
                        .insert(job);

                    to_descend_into.push(job);
                });

            to_convert.push(next_glue_job);
        }

        while let Some(glue_job) = to_convert.pop() {
            let job = job::Job::from_glue(glue_job, &glue_to_job_key)
                .context("could not convert glue job into actual job")?;

            if let Some(deps) = job_deps.get(glue_job) {
                let blockers = coordinator.blocked.entry(job.base_key).or_default();

                for dep in deps {
                    blockers.insert(
                        *glue_to_job_key
                            .get(dep)
                            .context("could not get job key for a glue job. This is probably an internal ordering bug and should be reported!")?
                    );
                }
            } else {
                coordinator.ready.push(job.base_key);
            }

            glue_to_job_key.insert(glue_job, job.base_key);
            coordinator.jobs.insert(job.base_key, job);
        }

        Ok(coordinator)
    }
}

#[derive(Debug)]
pub struct Coordinator<'roc> {
    workspace_root: PathBuf,
    store: Store,

    // caches
    path_to_hash: HashMap<PathBuf, String>,

    // note:  this mapping is only safe to use in the context of a single
    // execution since a job's final key may change without the base key
    // changing. Practically speaking, this just means you shouldn't store it!
    job_to_content_hash: HashMap<job::Key<job::Base>, store::Item>,

    // which jobs should run when?
    jobs: HashMap<job::Key<job::Base>, Job<'roc>>,
    blocked: HashMap<job::Key<job::Base>, HashSet<job::Key<job::Base>>>,
    ready: Vec<job::Key<job::Base>>,
}

impl<'roc> Coordinator<'roc> {
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
        //
        // TODO: it feels like much of the code between here and `finalize`
        //  would be better off living in KeyBuilder itself. It probably doesn't
        //  need to have a builder pattern at all, in fact, just a regular
        //  constructor.
        let mut key_builder = job::KeyBuilder::based_on(&job.base_key);
        for path in &job.input_files {
            match self.path_to_hash.get(path) {
                Some(hash) => key_builder.add_file(path, hash),
                None => anyhow::bail!("`{}` was specified as a file dependency, but I didn't have a hash for it! This is a bug in rbt's coordinator, please file it!", path.display()),
            }
        }
        for key in job.input_jobs.keys().sorted() {
            key_builder.add_dependency(&self.job_to_content_hash.get(key).context("could not look up output hash for dependency. This is a bug in rbt's coordinator. Please file it!")?.hash());
        }
        let key = key_builder.finalize();

        // build (or don't) based on the final key!
        match self
            .store
            .item_for_job(&key)
            .context("could not get a store path for the current job")?
        {
            Some(item) => {
                log::debug!("already had output of job {}; skipping", job);
                self.job_to_content_hash.insert(job.base_key, item);
            }
            None => {
                let workspace = Workspace::create(&self.workspace_root, &key)
                    .with_context(|| format!("could not create workspace for {}", job))?;

                workspace
                    .set_up_files(job, &self.job_to_content_hash)
                    .with_context(|| format!("could not set up workspace files for {}", job))?;

                runner.run(job, &workspace).context("could not run job")?;

                self.job_to_content_hash.insert(
                    job.base_key,
                    self.store
                        .store_from_workspace(key, job, workspace)
                        .context("could not store job output")?,
                );
            }
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
