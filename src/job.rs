use crate::glue;
use itertools::Itertools;
use roc_std::{RocList, RocStr};
use std::hash::{Hash, Hasher};
use std::process::Command;

#[derive(Debug, Eq, Hash, PartialEq, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Id(u64);

impl From<&glue::Job> for Id {
    /// We don't care about order in some places (e.g. output file) while we do
    /// in others (e.g. command arguments.) The hash should reflect this!
    ///
    /// Note: this data structure is going to grow the ability to refer to other
    /// jobs as soon as it's feasible. When that happens, a depth-first search
    /// through the tree rooted at `top_job` will probably suffice.
    fn from(top_job: &glue::Job) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        let job = &top_job.f0;

        // TODO: when we can get commands from other jobs, we need to hash the
        // other tool and job instead of relying on the derived `Hash` trait
        // for this.
        job.command.hash(&mut hasher);

        job.inputFiles
            .iter()
            .sorted()
            .for_each(|input_file| input_file.hash(&mut hasher));

        job.outputs
            .iter()
            .sorted()
            .for_each(|output| output.hash(&mut hasher));

        Id(hasher.finish())
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

#[derive(Debug)]
pub struct Job {
    pub id: Id,
    pub command: glue::R3,
    pub input_files: RocList<RocStr>,
    pub outputs: RocList<RocStr>,
}

impl From<glue::Job> for Job {
    fn from(job: glue::Job) -> Self {
        let id = Id::from(&job);
        let unwrapped = job.f0;

        Job {
            id,
            command: unwrapped.command.f0,
            input_files: unwrapped.inputFiles,
            outputs: unwrapped.outputs,
        }
    }
}

impl From<&Job> for Command {
    fn from(job: &Job) -> Self {
        let mut command = Command::new(&job.command.tool.f0.to_string());

        for arg in &job.command.args {
            command.arg(arg.as_str());
        }

        command
    }
}
