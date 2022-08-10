use crate::bindings;
use roc_std::{RocList, RocStr};

#[derive(Debug)]
pub struct Rbt {
    default: Job,
}

impl From<bindings::Rbt> for Rbt {
    fn from(rbt: bindings::Rbt) -> Self {
        // let unwrapped = rbt.into_Rbt();
        let unwrapped = rbt.f0;

        Rbt {
            default: Job::from(unwrapped.default),
        }
    }
}

#[derive(Debug)]
pub struct Job {
    command: Command,
    inputFiles: RocList<RocStr>,
    outputs: RocList<RocStr>,
}

impl From<bindings::Job> for Job {
    fn from(job: bindings::Job) -> Self {
        // let unwrapped = job.into_Job();
        let unwrapped = job.f0;

        Job {
            command: Command::from(unwrapped.command),
            inputFiles: unwrapped.inputFiles,
            outputs: unwrapped.outputs,
        }
    }
}

#[derive(Debug)]
pub struct Command {
    tool: Tool,
    args: RocList<RocStr>,
}

impl From<bindings::Command> for Command {
    fn from(command: bindings::Command) -> Self {
        // let unwrapped = command.into_Job();
        let unwrapped = command.f0;

        Command {
            tool: Tool::from(unwrapped.tool),
            args: unwrapped.args,
        }
    }
}

#[derive(Debug)]
pub enum Tool {
    SystemTool(RocStr),
}

impl From<bindings::Tool> for Tool {
    fn from(tool: bindings::Tool) -> Self {
        Self::SystemTool(tool.f0)
    }
}
