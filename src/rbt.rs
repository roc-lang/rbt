use crate::bindings;
use roc_std::{RocList, RocStr};
use serde::ser::SerializeSeq;
use serde::Serialize;

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct Job {
    command: Command,
    #[serde(serialize_with = "serialize_roc_list_of_roc_str")]
    inputFiles: RocList<RocStr>,
    #[serde(serialize_with = "serialize_roc_list_of_roc_str")]
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

#[derive(Debug, Serialize)]
pub struct Command {
    tool: Tool,
    #[serde(serialize_with = "serialize_roc_list_of_roc_str")]
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

#[derive(Debug, Serialize)]
pub enum Tool {
    SystemTool {
        #[serde(serialize_with = "serialize_roc_str")]
        name: RocStr,
    },
}

impl From<bindings::Tool> for Tool {
    fn from(tool: bindings::Tool) -> Self {
        Self::SystemTool { name: tool.f0 }
    }
}

// Remote Types

fn serialize_roc_list_of_roc_str<S>(
    list: &RocList<RocStr>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut seq = serializer.serialize_seq(Some(list.len()))?;
    for item in list {
        seq.serialize_element(item.as_str())?;
    }
    seq.end()
}

fn serialize_roc_str<S>(roc_str: &RocStr, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(roc_str.as_str())
}
