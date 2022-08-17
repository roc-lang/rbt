use crate::glue;
use anyhow::{Context, Result};
use itertools::Itertools;
use roc_std::{RocList, RocStr};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::process::Command;

#[derive(Debug, Eq, Hash, PartialEq, Clone, Copy)]
pub struct JobId(u64);

impl From<u64> for JobId {
    fn from(unwrapped: u64) -> Self {
        JobId(unwrapped)
    }
}

impl From<&glue::Job> for JobId {
    fn from(job: &glue::Job) -> Self {
        JobId(hash_for_glue_job(job))
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

#[derive(Debug, Default)]
pub struct Coordinator<'job> {
    jobs: HashMap<JobId, RunnableJob<'job>>,
    blocked: HashMap<JobId, HashSet<JobId>>,
    ready: Vec<JobId>,
}

impl<'job> Coordinator<'job> {
    pub fn add_target(&mut self, top_job: glue::Job) {
        // Note: this data structure is going to grow the ability to refer to other
        // jobs as soon as it's possibly feasible. When that happens, a depth-first
        // search through the tree rooted at `top_job` will probably suffice.
        let job: RunnableJob = top_job.into();
        self.ready.push(job.id);
        self.jobs.insert(job.id, job);
    }

    pub fn has_outstanding_work(&self) -> bool {
        !self.blocked.is_empty() || !self.ready.is_empty()
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

        // Now that we're done running the job, we update our bookkeeping to
        // figure out what running that job just unblocked.
        //
        // As an implementation note, this will probably end up in a separate
        // function once we're running tasks in parallel!
        let mut newly_unblocked = vec![]; // avoiding mutating both fields of self in the loop below

        self.blocked.retain(|blocked, blockers| {
            let removed = blockers.remove(&next);
            if !removed {
                return false;
            }

            let no_blockers_remaining = blockers.is_empty();
            if no_blockers_remaining {
                newly_unblocked.push(*blocked)
            }
            !no_blockers_remaining
        });
        self.ready.extend(newly_unblocked);

        Ok(())
    }
}

#[derive(Debug)]
pub struct RunnableJob<'job> {
    pub id: JobId,
    pub command: glue::Command,
    pub inputs: HashMap<&'job str, JobId>,
    pub input_files: RocList<RocStr>,
    pub outputs: RocList<RocStr>,
}

impl<'job> From<glue::Job> for RunnableJob<'job> {
    fn from(job: glue::Job) -> Self {
        let id = JobId::from(&job);
        let unwrapped = job.f0;

        RunnableJob {
            id,
            command: unwrapped.command,
            inputs: HashMap::default(),
            input_files: unwrapped.inputFiles,
            outputs: unwrapped.outputs,
        }
    }
}

impl<'job> From<&RunnableJob<'job>> for Command {
    fn from(job: &RunnableJob) -> Self {
        let mut command = Command::new(&job.command.f0.tool.f0.to_string());

        for arg in &job.command.f0.args {
            command.arg(arg.as_str());
        }

        command
    }
}

pub trait Runner {
    fn run(&self, job: &RunnableJob) -> Result<()>;
}

impl Runner for Box<dyn Runner> {
    fn run(&self, job: &RunnableJob) -> Result<()> {
        self.as_ref().run(job)
    }
}

/// Some parts of `glue::Job` do not have a meaningful ordering (for example,
/// the order of output files) while some do (for example, the ordering of
/// command arguments.) This hasher's job is to return the same value for a
/// non-meaningful change, but change immediately for a meaningful one.
///
/// Note: this data structure is going to grow the ability to refer to other
/// jobs as soon as it's possibly feasible. When that happens, a depth-first
/// search through the tree rooted at `top_job` will probably suffice.
fn hash_for_glue_job(top_job: &glue::Job) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();

    let job = &top_job.f0;

    // TODO: when we can get commands from other jobs, we need to hash the
    // other tool instead of relying on the derived `Hash` trait for this,
    // for the reasons in the top doc comment here.
    job.command.hash(&mut hasher);

    job.inputFiles
        .iter()
        .sorted()
        .for_each(|input_file| input_file.hash(&mut hasher));

    job.outputs
        .iter()
        .sorted()
        .for_each(|output| output.hash(&mut hasher));

    hasher.finish()
}
