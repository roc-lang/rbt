use crate::glue;
use crate::job::{self, Job};
use crate::path_meta_key::PathMetaKey;
use crate::runner::RunnerBuilder;
use crate::store::{self, Store};
use crate::workspace::Workspace;
use anyhow::{Context, Result};
use core::convert::TryInto;
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use tokio::task::JoinHandle;
use xxhash_rust::xxh3::Xxh3Builder;

pub struct Builder<'roc> {
    store: Store,
    roots: Vec<&'roc glue::Job>,
    meta_to_hash: sled::Tree,
    workspace_root: PathBuf,
    max_local_jobs: NonZeroUsize,
}

impl<'roc> Builder<'roc> {
    pub fn new(
        store: Store,
        meta_to_hash: sled::Tree,
        workspace_root: PathBuf,
        max_local_jobs: NonZeroUsize,
    ) -> Self {
        Builder {
            store,
            meta_to_hash,
            workspace_root,
            max_local_jobs,

            // it's very likely we'll have at least one root
            roots: Vec::with_capacity(1),
        }
    }

    pub fn add_root(&mut self, job: &'roc glue::Job) {
        self.roots.push(job);
    }

    pub fn build(self) -> Result<Coordinator> {
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

        // We assume that there will be at least some overlap in inputs (i.e. at
        // least two targets needing the same file.) That assumption means that
        // it makes sense to deduplicate them to avoid duplicating filesystem
        // operations.
        let mut input_files: HashSet<PathBuf> = HashSet::new();
        for glue_job in &self.roots {
            for input in &glue_job.as_Job().inputs {
                if input.discriminant() == glue::discriminant_U1::FromProjectSource {
                    for glue::FileMapping { source, .. } in unsafe { input.as_FromProjectSource() }
                    {
                        input_files.insert(job::sanitize_file_path(source)?);
                    }
                }
            }
        }

        let mut coordinator = Coordinator {
            store: self.store,
            roots: Vec::with_capacity(self.roots.len()),
            max_local_jobs: self.max_local_jobs.get(),

            path_to_hash: HashMap::with_capacity(input_files.len()),
            job_to_content_hash: HashMap::with_capacity(self.roots.len()),
            final_keys: HashMap::with_capacity(self.roots.len()),

            // On capacities: we'll have at least as many jobs as we have targets,
            // each of which will have at least one leaf node.
            jobs: HashMap::with_capacity(self.roots.len()),
            blocked: HashMap::default(),

            ready: Vec::with_capacity(self.roots.len()),
            running: FuturesUnordered::new(),

            // TODO: clean up bits of state
            runner_builder: RunnerBuilder::new(self.workspace_root.clone()),
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

        //////////////////////////////////////////////////////////////////
        // Phase 2: get hashes for metadata keys we haven't seen before //
        //////////////////////////////////////////////////////////////////
        let mut hasher = blake3::Hasher::new();

        for (path, cache_key) in path_to_meta.iter() {
            let key = cache_key.to_db_key();
            if let Some(value) = self
                .meta_to_hash
                .get(key)
                .context("could not read file hash from database")?
            {
                let bytes: [u8; 32] = value
                    .as_ref()
                    .try_into()
                    .context("value was not exactly 32 bytes")?;

                coordinator
                    .path_to_hash
                    .insert(path.to_path_buf(), blake3::Hash::from(bytes));

                continue;
            }

            let mut file = File::open(path)
                .with_context(|| format!("couldn't open `{}` for hashing.", path.display()))?;

            hasher.reset();

            // The docs for Blake3 say that a 16 KiB buffer is the most
            // efficient (for SIMD reasons)
            let mut buf = [0; 16 * 1024];
            loop {
                let bytes = file.read(&mut buf)?;
                if bytes == 0 {
                    break;
                }
                hasher.update(&buf[0..bytes]);
            }

            let hash = hasher.finalize();

            log::debug!("hash of `{}` was {}", path.display(), hash);
            log::trace!("bytes of hash: {:?}", hash.as_bytes());
            self.meta_to_hash
                .insert(key, hash.as_bytes())
                .context("could not write file hash to database")?;

            coordinator.path_to_hash.insert(path.to_path_buf(), hash);
        }

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
        // and `to_convert` tracks the dependencies in root-to-leaf order.
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
            // multiple jobs can depend on the same job, but we only need to
            // convert each job once.
            if let Some(key) = glue_to_job_key.get(glue_job) {
                log::trace!("already converted job {}", key);
                continue;
            }

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

        // we couldn't track which roots were needed before because we didn't
        // have the keys for those jobs. Now that we do, take a minute to
        // populate the roots vec (which up until now has had the right capacity
        // but no items.)
        for root in self.roots {
            coordinator.roots.push(
                *glue_to_job_key
                    .get(root)
                    .context("could not key for root job")?,
            )
        }

        Ok(coordinator)
    }
}

type DoneMsg = (job::Key<job::Base>, Option<Workspace>);

#[derive(Debug)]
pub struct Coordinator {
    store: Store,
    runner_builder: RunnerBuilder,

    roots: Vec<job::Key<job::Base>>,
    max_local_jobs: usize,

    // caches
    path_to_hash: HashMap<PathBuf, blake3::Hash>,
    final_keys: HashMap<job::Key<job::Base>, job::Key<job::Final>>,

    // note:  this mapping is only safe to use in the context of a single
    // execution since a job's final key may change without the base key
    // changing. Practically speaking, this just means you shouldn't store it!
    job_to_content_hash: HashMap<job::Key<job::Base>, store::Item>,

    // which jobs should run when?
    jobs: HashMap<job::Key<job::Base>, Job>,
    blocked: HashMap<job::Key<job::Base>, HashSet<job::Key<job::Base>>>,

    // what's the state of the coordinator while running?
    ready: Vec<job::Key<job::Base>>,
    running: FuturesUnordered<JoinHandle<Result<DoneMsg>>>,
}

impl<'roc> Coordinator {
    /// Run the build from start to finish.
    pub async fn run(&mut self) -> Result<()> {
        log::trace!("scheduling immediately-available jobs");
        self.schedule()
            .await
            .context("could not start immediately-ready jobs")?;

        let mut failed = false;

        log::trace!("starting coordinator loop");
        while let Some(join_res) = self.running.next().await {
            match join_res {
                Ok(Ok(done_msg)) => self
                    .handle_done(done_msg)
                    .await
                    .context("could not finish job")?,
                Ok(Err(err)) => {
                    log::error!("{:?}", err.context("job failed"));
                    failed = true
                }
                Err(err) => {
                    log::error!(
                        "{:?}",
                        anyhow::Error::new(err).context("could not join async task")
                    );
                    failed = true
                }
            }
        }

        if failed {
            anyhow::bail!("there was a failure while building; see logs for details")
        } else {
            Ok(())
        }
    }

    /// Start any outstanding work according to our scheduling rules. Right
    /// now that just means that we won't ever be running more jobs than
    /// `self.max_local_jobs`.
    async fn schedule(&mut self) -> Result<()> {
        let maximum_schedulable = self.max_local_jobs.saturating_sub(self.running.len());

        // The intent here is to drain a certain number of items from
        // `self.ready`. If the borrowing rules allowed it, we'd drain directly.
        let mut ready_now = self
            .ready
            .split_off(self.ready.len() - maximum_schedulable.min(self.ready.len()));

        log::debug!("scheduling {} jobs", ready_now.len());
        for id in ready_now.drain(..) {
            self.start(id)
                .await
                .context("could not start job from immediately-available set")?;
        }

        Ok(())
    }

    /// Start and track a single job by ID.
    async fn start(&mut self, id: job::Key<job::Base>) -> Result<()> {
        let job = self.jobs.get(&id).context("had a bad job ID")?;

        log::debug!("preparing to run job {}", job);

        let final_key = job
            .final_key(&self.path_to_hash, &self.job_to_content_hash)
            .context("could not calculate final cache key")?;
        self.final_keys.insert(id, final_key);

        // build (or don't) based on the final key!
        let join_handle = match self
            .store
            .item_for_job(&final_key)
            .context("could not get a store path for the current job")?
        {
            Some(item) => {
                log::debug!("already had output of job {}; skipping", job);
                self.job_to_content_hash.insert(job.base_key, item);

                tokio::spawn(async move { Ok((id, None)) })
            }
            None => {
                // TODO:  this preparation step probably represents a
                // bottleneck. In the current design, we need to be able to
                // access `job_to_content_hash` to prepare the workspace. It's
                // not send-safe, so we either need to copy only the keys we
                // need for the current job or use some data structure that
                // is sendable.
                //
                // Doing that would also mean that we could move preparation
                // into the spawned task, which would remove the requirement
                // that `start` be `async` (at least as of the writing of this
                // comment.)
                let runner = self
                    .runner_builder
                    .build(job, &self.job_to_content_hash)
                    .await
                    .context("could not prepare job to run")?;

                tokio::spawn(async move {
                    let workspace = runner.run().await.context("could not run job")?;

                    Ok((id, Some(workspace)))
                })
            }
        };

        self.running.push(join_handle);

        Ok(())
    }

    async fn handle_done(&mut self, msg: DoneMsg) -> Result<()> {
        let (id, workspace_opt) = msg;

        let job = self.jobs.get(&id).context("had a bad job ID")?;

        let final_key = self
            .final_keys
            .get(&id)
            .context("could not retrieve final cache key; was it calculated in `start`?")?;

        if let Some(workspace) = workspace_opt {
            self.job_to_content_hash.insert(
                job.base_key,
                self.store
                    .store_from_workspace(*final_key, job, workspace)
                    .await
                    .context("could not store job output")?,
            );
        };

        // Now that we're done running the job, we update our bookkeeping to
        // figure out what running that job just unblocked.
        let mut newly_unblocked = vec![]; // get around needing an async context in the loop below

        self.blocked.retain(|blocked, blockers| {
            let removed = blockers.remove(&id);
            if !removed {
                return true;
            }

            let no_blockers_remaining = blockers.is_empty();
            if no_blockers_remaining {
                log::debug!("unblocked {}", blocked);
                newly_unblocked.push(*blocked);
            }
            !no_blockers_remaining
        });

        for id in newly_unblocked.drain(..) {
            self.ready.push(id)
        }

        self.schedule().await.context("could not start new jobs")?;

        Ok(())
    }

    pub fn roots(&self) -> &[job::Key<job::Base>] {
        self.roots.as_ref()
    }

    pub fn store_path(&self, key: &job::Key<job::Base>) -> Option<&store::Item> {
        self.job_to_content_hash.get(key)
    }
}
