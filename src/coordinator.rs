use crate::glue;
use crate::job::{self, Job};
use crate::path_meta_key::PathMetaKey;
use crate::runner::RunnerBuilder;
use crate::store::{self, Store};
use anyhow::{Context, Result};
use core::convert::TryInto;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use xxhash_rust::xxh3::Xxh3Builder;

pub struct Builder<'roc> {
    store: Store,
    roots: Vec<&'roc glue::Job>,
    meta_to_hash: sled::Tree,
    workspace_root: PathBuf,
}

impl<'roc> Builder<'roc> {
    pub fn new(store: Store, meta_to_hash: sled::Tree, workspace_root: PathBuf) -> Self {
        Builder {
            store,
            meta_to_hash,
            workspace_root,

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
                    for file in unsafe { input.as_FromProjectSource() } {
                        input_files.insert(job::sanitize_file_path(file)?);
                    }
                }
            }
        }

        let mut coordinator = Coordinator {
            store: self.store,
            workspace_root: self.workspace_root,
            roots: Vec::with_capacity(self.roots.len()),

            path_to_hash: HashMap::with_capacity(input_files.len()),
            job_to_content_hash: HashMap::with_capacity(self.roots.len()),

            // On capacities: we'll have at least as many jobs as we have targets,
            // each of which will have at least one leaf node.
            jobs: HashMap::with_capacity(self.roots.len()),
            blocked: HashMap::default(),
            ready_immediately: Vec::with_capacity(self.roots.len()),
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
            if let Some(hash) = self
                .meta_to_hash
                .get(&key)
                .context("could not read file hash from database")?
            {
                coordinator.path_to_hash.insert(
                    path.to_path_buf(),
                    std::str::from_utf8(hash.as_ref())?.into(),
                );

                continue;
            }

            let mut file = File::open(&path)
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
            self.meta_to_hash
                .insert(key, hash.as_bytes())
                .context("could not write file hash to database")?;

            coordinator
                .path_to_hash
                .insert(path.to_path_buf(), hash.to_string());
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
                coordinator.ready_immediately.push(job.base_key);
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

#[derive(Debug)]
pub struct Coordinator {
    store: Store,
    workspace_root: PathBuf,

    roots: Vec<job::Key<job::Base>>,

    // caches
    path_to_hash: HashMap<PathBuf, String>,

    // note:  this mapping is only safe to use in the context of a single
    // execution since a job's final key may change without the base key
    // changing. Practically speaking, this just means you shouldn't store it!
    job_to_content_hash: HashMap<job::Key<job::Base>, store::Item>,

    // which jobs should run when?
    jobs: HashMap<job::Key<job::Base>, Job>,
    blocked: HashMap<job::Key<job::Base>, HashSet<job::Key<job::Base>>>,

    // TODO: should this be calculated based on the keys that are in `jobs` but
    // not in `blocked`?
    ready_immediately: Vec<job::Key<job::Base>>,
}

impl<'roc> Coordinator {
    pub async fn run_all(&mut self) -> Result<()> {
        let runner_builder = RunnerBuilder::new(self.workspace_root.clone());

        while self.has_outstanding_work() {
            let id = match self.ready_immediately.pop() {
                Some(id) => id,
                None => anyhow::bail!("no work ready to do"),
            };

            let job = self
                .jobs
                .get(&id)
                .context("had a bad job ID in Coordinator.ready")?;

            log::debug!("preparing to run job {}", job);

            let final_key = job
                .final_key(&self.path_to_hash, &self.job_to_content_hash)
                .context("could not calculate final cache key")?;

            // build (or don't) based on the final key!
            match self
                .store
                .item_for_job(&final_key)
                .context("could not get a store path for the current job")?
            {
                Some(item) => {
                    log::debug!("already had output of job {}; skipping", job);
                    self.job_to_content_hash.insert(job.base_key, item);
                }
                None => {
                    let runner = runner_builder
                        .build(job, &self.job_to_content_hash)
                        .await
                        .context("could not prepare job to run")?;

                    let workspace = runner.run().await.context("could not run job")?;

                    self.job_to_content_hash.insert(
                        job.base_key,
                        self.store
                            .store_from_workspace(final_key, job, workspace)
                            .await
                            .context("could not store job output")?,
                    );
                }
            }

            // Now that we're done running the job, we update our bookkeeping to
            // figure out what running that job just unblocked.
            let mut newly_unblocked = vec![]; // avoiding mutating both fields of self in the loop below

            self.blocked.retain(|blocked, blockers| {
                let removed = blockers.remove(&id);
                if !removed {
                    return true;
                }

                let no_blockers_remaining = blockers.is_empty();
                if no_blockers_remaining {
                    log::debug!("unblocked {}", blocked);
                    newly_unblocked.push(*blocked)
                }
                !no_blockers_remaining
            });
            self.ready_immediately.extend(newly_unblocked);
        }

        Ok(())
    }

    pub fn has_outstanding_work(&self) -> bool {
        !self.blocked.is_empty() || !self.ready_immediately.is_empty()
    }

    pub fn roots(&self) -> &[job::Key<job::Base>] {
        self.roots.as_ref()
    }

    pub fn store_path(&self, key: &job::Key<job::Base>) -> Option<&store::Item> {
        self.job_to_content_hash.get(key)
    }
}
