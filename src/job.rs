use crate::glue;
use roc_std::{RocList, RocStr};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::process::Command;

#[derive(Debug, Eq, Hash, PartialEq, Clone, Copy)]
pub struct Id(u64);

impl From<u64> for Id {
    fn from(unwrapped: u64) -> Self {
        Id(unwrapped)
    }
}

impl From<&glue::Job> for Id {
    fn from(job: &glue::Job) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        job.hash(&mut hasher);

        Id(hasher.finish())
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

#[derive(Debug)]
pub struct Job<'job> {
    pub id: Id,
    pub command: glue::R3,
    pub inputs: HashMap<&'job str, Id>,
    pub input_files: RocList<RocStr>,
    pub outputs: RocList<RocStr>,
}

impl<'job> From<glue::Job> for Job<'job> {
    fn from(job: glue::Job) -> Self {
        let id = Id::from(&job);
        let unwrapped = job.f0;

        Job {
            id,
            command: unwrapped.command.f0,
            inputs: HashMap::default(),
            input_files: unwrapped.inputFiles,
            outputs: unwrapped.outputs,
        }
    }
}

impl<'job> From<&Job<'job>> for Command {
    fn from(job: &Job) -> Self {
        let mut command = Command::new(&job.command.tool.f0.to_string());

        for arg in &job.command.args {
            command.arg(arg.as_str());
        }

        command
    }
}
