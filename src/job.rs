use crate::glue;
use anyhow::{Context, Result};
use itertools::Itertools;
use roc_std::RocStr;
use std::collections::HashSet;
use std::fmt::{self, Display};
use std::hash::{Hash, Hasher};
use std::path::{Component, PathBuf};
use std::process::Command;
use xxhash_rust::xxh3::Xxh3;

#[derive(Debug, Eq, Hash, PartialEq, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Key(u64);

impl Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

#[derive(Debug)]
pub struct Job {
    pub id: Key,
    pub command: glue::CommandPayload,
    pub input_files: HashSet<PathBuf>,
    pub outputs: HashSet<PathBuf>,
}

impl Job {
    pub fn from_glue(job: glue::Job) -> Result<Self> {
        let unwrapped = job.into_Job();

        let mut hasher = Xxh3::new();

        // TODO: when we can get commands from other jobs, we need to hash the
        // other tool and job instead of relying on the derived `Hash` trait
        // for this.
        unwrapped.command.hash(&mut hasher);

        let mut input_files: HashSet<PathBuf> = HashSet::with_capacity(unwrapped.inputFiles.len());
        for path_str in unwrapped.inputFiles.iter().sorted() {
            let path =
                sanitize_file_path(path_str).context("got an unacceptable input file path")?;

            path.hash(&mut hasher);
            input_files.insert(path);
        }

        let mut outputs = HashSet::new();
        for output_str in unwrapped.outputs.iter().sorted() {
            let output =
                sanitize_file_path(output_str).context("got an unacceptable output file path")?;

            if outputs.contains(&output) {
                log::warn!(
                    "`{}` appears twice in the list of outputs",
                    output.display()
                );
                continue;
            }

            output.hash(&mut hasher);
            outputs.insert(output);
        }

        Ok(Job {
            id: Key(hasher.finish()),
            command: unwrapped.command.into_Command(),
            input_files,
            outputs,
        })
    }
}

impl From<&Job> for Command {
    fn from(job: &Job) -> Self {
        let mut command = Command::new(&job.command.tool.as_SystemTool().to_string());

        for arg in &job.command.args {
            command.arg(arg.as_str());
        }

        command
    }
}

impl Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // intention: make a best-effort version of part of how the command
        // would look if it were invoked from a shell. It's OK for right now
        // if it wouldn't work (due to unescaped quotes or whatever.) Point is
        // for us to have some human-readable output in addition to the ID.
        let mut chars = 0;

        write!(f, "{} (", self.id)?;

        let base = self.command.tool.as_SystemTool().to_string();
        chars += base.len();

        write!(f, "{}", base)?;

        for arg in &self.command.args {
            if chars >= 20 {
                continue;
            }

            if arg.contains(' ') {
                write!(f, " \"{}\"", arg)?;
                chars += arg.len() + 3;
            } else {
                write!(f, " {}", arg)?;
                chars += arg.len() + 1;
            }
        }

        write!(f, ")")
    }
}

pub fn sanitize_file_path(roc_str: &RocStr) -> Result<PathBuf> {
    let sanitized: PathBuf = roc_str.as_str().into();

    // verify that the specified path is safe. We can't allow accessing any
    // path outside the workspace. To get this, we don't allow any parent path
    // segments (`..`) This restriction also enforces unambiguous paths in the
    // Roc API (e.g. you wouldn't want to add "foo/../bar" as an output path!)
    for component in sanitized.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => anyhow::bail!(
                "Absolute paths like `{}` are not allowed. Remove the absolute prefix to fix this!",
                sanitized.display(),
            ),

            Component::ParentDir => anyhow::bail!(
                "Relative paths containing `..` (like `{}`) are not allowed. Remove the `..` to fix this!",
                sanitized.display(),
            ),

            Component::CurDir | Component::Normal(_) => (),
        };
    }

    Ok(sanitized)
}

#[cfg(test)]
mod test {
    use super::*;
    use roc_std::RocList;

    #[test]
    fn job_hash_stability() {
        // It's important that job IDs don't change accidentally over time. For
        // example, if we update a dependency and the hash here suddenly changes,
        // we should look into it and consider a smooth migration path for
        // callers. Similarly, it might be inappropriate new optional fields in the
        // Roc API to contribute to the ID, since doing so would mean completely
        // re-running all build steps.
        let glue_job = glue::Job::Job(glue::JobPayload {
            command: glue::Command::Command(glue::CommandPayload {
                tool: glue::Tool::SystemTool(RocStr::from("bash")),
                args: RocList::from_slice(&["-c".into(), "Hello, World".into()]),
            }),
            inputFiles: RocList::from_slice(&["input_file".into()]),
            outputs: RocList::from_slice(&["ouput_file".into()]),
        });

        let mut path_to_hash: HashMap<PathBuf, String> = HashMap::new();
        path_to_hash.insert("input_file".into(), "hashhashhash".into());

        let job = Job::from_glue(glue_job, &path_to_hash).unwrap();

        assert_eq!(Key(9236546748343508395), job.id);
    }
}
