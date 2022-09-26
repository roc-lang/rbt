use crate::glue;
use anyhow::{Context, Result};
use itertools::Itertools;
use roc_std::{RocDict, RocStr};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display};
use std::hash::{BuildHasher, Hash, Hasher};
use std::marker::PhantomData;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use xxhash_rust::xxh3::Xxh3;

/// Conversion from a base key to a final one.
pub struct KeyBuilder(Xxh3);

impl KeyBuilder {
    fn new() -> Self {
        Self(Xxh3::new())
    }

    #[cfg(test)]
    pub fn mock() -> Self {
        Self::new()
    }

    pub fn based_on(id: &Key<Base>) -> Self {
        let mut builder = Self::new();
        id.hash(&mut builder.0);

        builder
    }

    pub fn add_file(&mut self, path: &Path, content_hash: &str) {
        path.hash(&mut self.0);
        content_hash.hash(&mut self.0);
    }

    pub fn finalize(self) -> Key<Final> {
        Key {
            key: self.0.finish(),
            phantom: PhantomData,
        }
    }
}

/// See docs on `Key`
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Deserialize)]
pub struct Base;

/// See docs on `Key`
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Deserialize)]
pub struct Final;

/// A cache key for a job. This has a phantom type parameter because we calculate
/// cache keys over multipole stages. The first (corresponding to `Base`) is just
/// the information passed in from a `glue::Job`. The second includes information
/// we'd have to do I/O for (like file hashes.) For more on the architecture,
/// see `docs/internals/how-we-determine-when-to-run-jobs.md`.
#[derive(Debug, Eq, Hash, PartialEq, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Key<Finality> {
    key: u64,
    phantom: PhantomData<Finality>,
}

impl<Finality> Display for Key<Finality> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.key)
    }
}

#[derive(Debug)]
pub struct Job<'roc> {
    pub base_key: Key<Base>,
    pub command: &'roc glue::Command,
    pub env: &'roc RocDict<RocStr, RocStr>,
    pub input_files: HashSet<PathBuf>,
    pub input_jobs: HashMap<Key<Base>, HashSet<PathBuf>>,
    pub outputs: HashSet<PathBuf>,
}

impl<'roc> Job<'roc> {
    pub fn from_glue<S>(
        job: &'roc glue::Job,
        glue_job_to_key: &HashMap<&glue::Job, Key<Base>, S>,
    ) -> Result<Self>
    where
        S: BuildHasher,
    {
        let unwrapped = job.as_Job();

        let mut hasher = Xxh3::new();

        // TODO: when we can get commands from other jobs, we need to hash the
        // other tool and job instead of relying on the derived `Hash` trait
        // for this.
        unwrapped.command.hash(&mut hasher);

        let mut input_files: HashSet<PathBuf> = HashSet::new();
        let mut input_jobs: HashMap<Key<Base>, HashSet<PathBuf>> = HashMap::new();

        for input in unwrapped.inputs.iter().sorted() {
            match input.discriminant() {
                glue::discriminant_U1::FromJob => {
                    let (glue_job, files) = unsafe { input.as_FromJob() };

                    // note that we're not hashing this key. We'll hash the
                    // content hash from the dependency job later, so we're
                    // getting this information anyway, and hashing it here
                    // would cause a rebuild on any source change in the
                    // dependent job, even (for example) a comment moving
                    // around.
                    let key = glue_job_to_key.get(glue_job).context("could not get job key to determine build order. This indicates an internal bug in the coordinator module and should be reported.")?;
                    let mut job_files = HashSet::new();

                    for file in files {
                        let path = sanitize_file_path(file)
                            .context("got an unnacceptable input file path")?;

                        // TODO: when we have mapped filenames, both components
                        // of the mapped file name should be added to the hash
                        // here. (See ADR 008)
                        path.hash(&mut hasher);
                        job_files.insert(path);
                    }

                    input_jobs.insert(*key, job_files);
                }
                glue::discriminant_U1::FromProjectSource => {
                    for file in unsafe { input.as_FromProjectSource() }.iter().sorted() {
                        let path = sanitize_file_path(file)
                            .context("got an unacceptable input file path")?;

                        path.hash(&mut hasher);
                        input_files.insert(path);
                    }
                }
            }
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

        for (key, value) in unwrapped.env.iter().sorted() {
            key.hash(&mut hasher);
            value.hash(&mut hasher);
        }

        Ok(Job {
            base_key: Key {
                key: hasher.finish(),
                phantom: PhantomData,
            },
            env: &unwrapped.env,
            command: &unwrapped.command,
            input_files,
            input_jobs,
            outputs,
        })
    }
}

impl<'roc> From<&Job<'roc>> for Command {
    fn from(job: &Job) -> Self {
        let mut command = Command::new(&job.command.tool.as_SystemTool().name.to_string());

        for arg in &job.command.args {
            command.arg(arg.as_str());
        }

        for (key, value) in job.env {
            command.env(key.as_str(), value.as_str());
        }

        command
    }
}

impl<'roc> Display for Job<'roc> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // intention: make a best-effort version of part of how the command
        // would look if it were invoked from a shell. It's OK for right now
        // if it wouldn't work (due to unescaped quotes or whatever.) Point is
        // for us to have some human-readable output in addition to the ID.
        let mut chars = 0;

        write!(f, "{} (", self.base_key)?;

        let base = self.command.tool.as_SystemTool().name.to_string();
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
        let glue_job = glue::Job::Job(glue::R1 {
            command: glue::Command {
                tool: glue::Tool::SystemTool(glue::SystemToolPayload {
                    name: RocStr::from("bash"),
                }),
                args: RocList::from_slice(&["-c".into(), "Hello, World".into()]),
            },
            env: RocDict::with_capacity(0),
            inputs: RocList::from_slice(&[glue::U1::FromProjectSource(RocList::from([
                "input_file".into(),
            ]))]),
            outputs: RocList::from_slice(&["output_file".into()]),
        });

        let job = Job::from_glue(&glue_job, &HashMap::new()).unwrap();

        assert_eq!(
            Key {
                key: 243796661244433339,
                phantom: PhantomData
            },
            job.base_key
        );
    }
}
