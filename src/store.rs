use crate::job::{self, Job};
use crate::workspace::Workspace;
use anyhow::{Context, Result};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

/// This struct manages all the levels of storage that we need in order to avoid
/// doing as much work as possible. This mostly involves managing several layers
/// of caches:
#[derive(Debug)]
pub struct Store {
    root: PathBuf,

    // This is stored as JSON for now to avoid taking another dependency,
    // but it'd be good for it to be a real database (or database table)
    // eventually. SQLite or Sled or something
    inputs_to_content: HashMap<job::Id, String>,
}

impl Store {
    pub fn new(root: PathBuf) -> Result<Self> {
        let inputs_to_content = match File::open(&root.join("inputs_to_content.json")) {
            Ok(file) => {
                let reader = BufReader::new(file);
                serde_json::from_reader(reader)
                    .context("could not deserialize mapping from inputs to content")?
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => HashMap::default(),
            Err(err) => return Err(err).context("could not open mapping from inputs to content"),
        };

        if !root.exists() {
            log::info!("creating store root at {}", &root.display());
            std::fs::create_dir_all(&root).context("could not create specified root")?;
        }

        Ok(Store {
            root,
            inputs_to_content,
        })
    }

    pub fn for_job(&self, job: &Job) -> Option<PathBuf> {
        self.inputs_to_content
            .get(&job.id)
            .map(|path| self.root.join(path))
    }

    pub fn store_from_workspace(&mut self, job: &Job, workspace: Workspace) -> Result<()> {
        let output = Output::load(job, workspace).context("could get output from job")?;

        if output.exists_in(&self.root) {
            log::debug!(
                "we have already stored {}, so I'm skipping the move!",
                output,
            );
        } else {
            log::debug!("moving {} into store", output);

            output
                .move_into(&self.root)
                .with_context(|| format!("could not move {} into the store", output))?;
        }

        self.associate_job_with_hash(job, &output.to_string())
            .context("could not associate job with hash")
    }

    fn associate_job_with_hash(&mut self, job: &Job, hash: &str) -> Result<()> {
        self.inputs_to_content.insert(job.id, hash.to_owned());

        let file = std::fs::File::create(self.root.join("inputs_to_content.json"))
            .context("failed to open job-to-content-hash mapping")?;
        // TODO: BufWriter?
        serde_json::to_writer(file, &self.inputs_to_content)
            .context("failed to write job-to-content-hash mapping")
    }
}

#[derive(Debug)]
struct Output<'job> {
    hasher: blake3::Hasher,
    workspace: Workspace,
    job: &'job Job,
}

impl<'job> Output<'job> {
    fn load(job: &'job Job, workspace: Workspace) -> Result<Self> {
        let mut output = Output {
            hasher: blake3::Hasher::new(),
            workspace,
            job,
        };

        for path in job.outputs.iter().sorted() {
            output.hasher.update(path.to_string_lossy().as_bytes());

            let mut file = File::open(&output.workspace.join(path)).with_context(|| {
                format!(
                    "couldn't open `{}` for hashing. Did the build produce it?",
                    path.display()
                )
            })?;

            // TODO: docs for Blake3 say that a 16 KiB buffer is the most
            // efficient (for SIMD reasons), but `std::io::copy` uses an 8KiB
            // buffer. Gonna have to do this by hand at some point to take
            // advantage of the algorithm's designed speed.
            std::io::copy(&mut file, &mut output.hasher).with_context(|| {
                format!("could not read `{}` to calculate hash", path.display())
            })?;
        }

        Ok(output)
    }

    fn final_path(&self, root: &Path) -> PathBuf {
        root.join(self.to_string())
    }

    fn exists_in(&self, root: &Path) -> bool {
        self.final_path(root).exists()
    }

    fn move_into(&self, root: &Path) -> Result<()> {
        let final_path = self.final_path(root);

        let temp = tempfile::Builder::new()
            .prefix(&format!("rbt-job-{}", self.job.id))
            .tempdir()
            .context("couldn't create temporary directory for hashing")?;

        let mut output_dirs: HashSet<PathBuf> = HashSet::new();

        for output in self.job.outputs.iter().sorted() {
            ///////////////////////////////////////////////
            // Step 2: make sure any parent paths exist  //
            ///////////////////////////////////////////////
            let mut ancestors: Vec<&Path> = output.ancestors().skip(1).collect();
            ancestors.pop(); // we've made sure this is relative, so the first item is ""
            ancestors.reverse(); // go `[a, a/b, a/b/c]` instead of `[a/b/c, a/b, a]`

            for ancestor_path in ancestors {
                let ancestor = ancestor_path.to_path_buf();

                if output_dirs.contains(&ancestor) {
                    continue;
                }

                log::trace!(
                    "creating {} in {}",
                    &ancestor.display(),
                    temp.path().display()
                );
                std::fs::create_dir(temp.path().join(&ancestor)).with_context(|| {
                    format!(
                        "could not create ancestor `{}` for output `{}`",
                        ancestor.display(),
                        output.display(),
                    )
                })?;
                output_dirs.insert(ancestor);
            }

            //////////////////////////////
            // Step 3: collect the file //
            //////////////////////////////
            log::trace!("moving {} into collection path", &output.display());
            let out = temp.path().join(&output);
            std::fs::rename(self.workspace.join(&output), &out).with_context(|| {
                format!(
                    "could not move `{}` from workspace to store",
                    output.display()
                )
            })?;

            Self::make_readonly(&out).with_context(|| {
                format!(
                    "could not make `{}` read-only after moving into store",
                    out.display()
                )
            })?;
        }

        // Finish up by making sure everything is readonly and store it in the
        // final location.
        for dir in &output_dirs {
            Self::make_readonly(&temp.path().join(&dir)).with_context(|| {
                format!("could not make `{}` read-only in the store", dir.display(),)
            })?;
        }

        // important: at this point we need to take ownership of the tempdir so
        // that it doesn't get automatically removed when it's dropped. We've
        // so far avoided that to avoid leaving temporary directories laying
        // around in case of errors.
        std::fs::rename(temp.into_path(), &final_path)
            .context("could not move temporary collection dir into the store")?;
        Self::make_readonly(&final_path).context("could not make store path readonly")?;

        Ok(())
    }

    fn make_readonly(path: &Path) -> Result<()> {
        let mut perms = std::fs::metadata(&path)
            .context("could not get file metadata")?
            .permissions();

        perms.set_readonly(true);

        std::fs::set_permissions(&path, perms).context("could not set permissions")
    }
}

impl<'job> Display for Output<'job> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.hasher.finalize().fmt(f)
    }
}
