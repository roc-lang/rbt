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

#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;

#[derive(Debug)]
pub struct Coordinator {
    workspace_root: PathBuf,
    store: Store,

    // initialization phase
    targets: Vec<glue::Job>,

    // caches
    path_to_meta: HashMap<PathBuf, PathMetaKey>,
    meta_to_hash: HashMap<u64, String>,

    // which jobs should run when?
    jobs: HashMap<job::Id, Job>,
    blocked: HashMap<job::Id, HashSet<job::Id>>,
    ready: Vec<job::Id>,
}

impl Coordinator {
    pub fn new(workspace_root: PathBuf, store: Store) -> Result<Self> {
        let meta_to_hash = match File::open(&workspace_root.join("file_hashes.json")) {
            Ok(file) => {
                let reader = BufReader::new(file);
                serde_json::from_reader(reader)
                    .context("could not deserialize mapping from inputs to content")?
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => HashMap::default(),
            Err(err) => return Err(err).context("could not open file hash cache"),
        };

        Ok(Coordinator {
            workspace_root,
            store,

            targets: Vec::new(),

            path_to_meta: HashMap::default(),
            meta_to_hash,

            jobs: HashMap::default(),
            blocked: HashMap::default(),
            ready: Vec::default(),
        })
    }

    pub fn add_target(&mut self, job: glue::Job) {
        self.targets.push(job);
    }

    pub fn prepare_for_work(&mut self) -> Result<()> {
        let mut input_files: HashSet<PathBuf> = HashSet::new();

        for glue_job in &self.targets {
            for file in &glue_job.f0.inputFiles {
                input_files.insert(file.as_str().into());
            }
        }

        // TODO: perf hint for later: we could be doing this in parallel
        // using rayon
        for input_file in input_files.drain() {
            // TODO: collect errors instead of bailing immediately
            let meta = input_file.metadata().with_context(|| {
                format!("could not read metadata for `{}`", input_file.display())
            })?;

            if meta.is_dir() {
                anyhow::bail!(
                    "One of your jobs specifies `{}`, a directory, as a dependency. I can only handle files.",
                    input_file.display(),
                )
            };

            let cache_key = meta.try_into().with_context(|| {
                format!(
                    "could not calculate a cache key for `{}`",
                    input_file.display()
                )
            })?;

            self.path_to_meta.insert(input_file, cache_key);
        }

        /////////////////////////////////////////////////
        // get hashes for files we haven't seen before //
        /////////////////////////////////////////////////
        let mut hasher = blake3::Hasher::new();

        for (path, cache_key) in &self.path_to_meta {
            let key = u64::from(cache_key);
            if self.meta_to_hash.contains_key(&key) {
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

            log::debug!("hash of `{}` was {}", path.display(), hasher.finalize());
            self.meta_to_hash.insert(key, hasher.finalize().to_string());
        }

        let file_hashes = File::create(self.workspace_root.join("file_hashes.json"))
            .context("could not open file hash cache to store new hashes")?;
        // TODO: BufWriter?
        serde_json::to_writer(file_hashes, &self.meta_to_hash)
            .context("failed to write hash cache to disk")?;

        for glue_job in self.targets.drain(..) {
            // Note: this data structure is going to grow the ability to
            // refer to other jobs as soon as it's possibly feasible. When
            // that happens, a depth-first search through the tree rooted at
            // `glue_job` will probably suffice.
            let job: Job = glue_job.into();
            self.ready.push(job.id);
            self.jobs.insert(job.id, job);
        }

        Ok(())
    }

    pub fn has_outstanding_work(&self) -> bool {
        !self.blocked.is_empty() || !self.ready.is_empty()
    }

    pub fn run_next<R: Runner>(&mut self, runner: &R) -> Result<()> {
        debug_assert_eq!(
            self.targets.len(),
            0,
            "there were still unprocessed targets. Did `prepare_for_work` run?"
        );

        let id = match self.ready.pop() {
            Some(id) => id,
            None => anyhow::bail!("no work ready to do"),
        };

        let job = self
            .jobs
            .get(&id)
            .context("had a bad job ID in Coordinator.ready")?;

        log::debug!("preparing to run job {}", job);

        if self.store.for_job(job).is_none() {
            let workspace = Workspace::create(&self.workspace_root, job)
                .with_context(|| format!("could not create workspace for job {}", job.id))?;

            runner.run(job, &workspace).context("could not run job")?;

            self.store
                .store_from_workspace(job, workspace)
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
        let mut hasher = std::collections::hash_map::DefaultHasher::default();
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
