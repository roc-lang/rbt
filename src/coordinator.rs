use crate::glue;
use crate::job::{self, Job};
use crate::store::Store;
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct Coordinator {
    store: Store,

    jobs: HashMap<job::Id, Job>,
    blocked: HashMap<job::Id, HashSet<job::Id>>,
    ready: Vec<job::Id>,
}

impl Coordinator {
    pub fn new(store: Store) -> Self {
        Coordinator {
            store,

            jobs: HashMap::default(),
            blocked: HashMap::default(),
            ready: Vec::default(),
        }
    }

    pub fn add_target(&mut self, top_job: glue::Job) {
        // Note: this data structure is going to grow the ability to refer to other
        // jobs as soon as it's possibly feasible. When that happens, a depth-first
        // search through the tree rooted at `top_job` will probably suffice.
        let job: Job = top_job.into();
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

pub trait Runner {
    fn run(&self, job: &Job) -> Result<()>;
}

impl Runner for Box<dyn Runner> {
    fn run(&self, job: &Job) -> Result<()> {
        self.as_ref().run(job)
    }
}
