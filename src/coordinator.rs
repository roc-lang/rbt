use crate::rbt;
use anyhow::{Context, Result};
use roc_std::{RocList, RocStr};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

#[derive(Debug, Eq, Hash, PartialEq, Clone, Copy)]
struct JobId(u64);

impl From<u64> for JobId {
    fn from(unwrapped: u64) -> Self {
        JobId(unwrapped)
    }
}

#[derive(Debug, Default)]
pub struct Coordinator<'job> {
    jobs: HashMap<JobId, RunnableJob<'job>>,
    blocked: HashMap<JobId, HashSet<JobId>>,
    ready: Vec<JobId>,
}

impl<'job> Coordinator<'job> {
    pub fn add_target(&mut self, target_job: &'job rbt::Job) {
        let mut todo = vec![target_job];

        while let Some(job) = todo.pop() {
            // TODO: figure out the right hasher for our use case and use that instead
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            job.hash(&mut hasher);
            let id = hasher.finish().into();

            let runnable_job = RunnableJob {
                command: &job.command,
                inputs: job
                    .inputs
                    .iter()
                    .map(|(name, dep)| {
                        let mut dep_hasher = std::collections::hash_map::DefaultHasher::new();
                        dep.hash(&mut dep_hasher);

                        (name.as_str(), dep_hasher.finish().into())
                    })
                    .collect(),
                input_files: &job.input_files,
                outputs: &job.outputs,
            };

            let blockers: HashSet<JobId> = runnable_job.inputs.values().copied().collect();

            if blockers.is_empty() {
                self.ready.push(id);
            } else {
                self.blocked.insert(id, blockers);
            }

            self.jobs.insert(id, runnable_job);

            todo.append(&mut job.inputs.values().collect());
        }
    }

    pub fn has_outstanding_work(&self) -> bool {
        !self.blocked.is_empty() && !self.ready.is_empty()
    }

    pub fn run_next<R: Runner>(&mut self, runner: &R) -> Result<()> {
        let next = match self.ready.pop() {
            Some(id) => id,
            None => anyhow::bail!("no work ready to do"),
        };

        runner
            .run(
                self.jobs
                    .get(&next)
                    .context("had a bad job ID in Coordinator.ready")?,
            )
            .context("could not run job")?;

        for (blocked, blockers) in self.blocked.iter_mut() {
            if blockers.remove(&next) {
                if blockers.is_empty() {
                    self.ready.push(*blocked);

                    // TODO: it would be more performant to remove the
                    // newly-unblocked item from self.blocked, but there's
                    // already a mutable borrow. Possibly rearrange the code
                    // to do some mutable filtering thing.
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct RunnableJob<'job> {
    pub command: &'job rbt::Command,
    inputs: HashMap<&'job str, JobId>, // not pub because inputs will eventually be provided in Runner.run
    pub input_files: &'job RocList<RocStr>,
    pub outputs: &'job RocList<RocStr>,
}

pub trait Runner {
    fn run(&self, job: &RunnableJob) -> Result<()>;
}
