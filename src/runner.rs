use crate::rbt::Job;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

#[derive(Debug, Default)]
pub struct Runner<'job> {
    jobs: HashMap<u64, &'job Job>,
    waiting_for: HashMap<u64, HashSet<u64>>,
}

impl<'job> Runner<'job> {
    #[tracing::instrument(skip(target_job))] // job is quite a bit of info for the log!
    pub fn add_target(&mut self, target_job: &'job Job) {
        let mut todo = vec![target_job];

        while let Some(job) = todo.pop() {
            let _span = tracing::span!(tracing::Level::TRACE, "processing job").entered();

            // TODO: figure out the right hasher for our use case and use that instead
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            job.hash(&mut hasher);
            let id = hasher.finish();

            self.jobs.insert(id, job);
            self.waiting_for.insert(
                id,
                job.inputs
                    .values()
                    .map(|dep| {
                        let mut dep_hasher = std::collections::hash_map::DefaultHasher::new();
                        dep.hash(&mut dep_hasher);
                        dep_hasher.finish()
                    })
                    .collect(),
            );

            todo.append(&mut job.inputs.values().collect());
        }
    }
}
