use crate::job::{self, Job};
use crate::workspace::Workspace;
use anyhow::{Context, Result};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

/// Store is responsible for managing a content-addressed store below some path
/// and managing the associations between input job hashes and those paths.
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

    /// If an output exists for a job, what is it? If we don't have a stored
    /// output for the job, return `None`.
    pub fn for_job(&self, job: &Job) -> Option<PathBuf> {
        self.inputs_to_content
            .get(&job.id)
            .map(|path| self.root.join(path))
    }

    /// Figure out if we need to make a new content-addressable item from the
    /// job's output, then store it if necessary. After running this function,
    /// `to_job` should return the correct store path.
    ///
    /// Some assumptions we're making:
    ///
    ///  1. The job already ran successfully and left files for us in the
    ///     Workspace directory.
    ///  2. The caller has already checked `for_job`, and that we definitely
    ///     know we need to store the ouput.
    ///  3. All the paths in the Job's `output` field have been sanitized (that
    ///     is, they don't include any paths leading to the root or other
    ///     drives, or contain `..` elements that would take the path out of
    ///     the workspace root.)
    pub fn store_from_workspace(&mut self, job: &Job, workspace: Workspace) -> Result<()> {
        let item = ContentAddressedItem::load(job, workspace)
            .context("could get content addressable item from job")?;

        if item.exists_in(&self.root) {
            log::debug!("we have already stored {}, so I'm skipping the move!", item,);

            self.associate_job_with_hash(job, &item.hash().to_string())
                .context("could not associate job with hash")
        } else {
            log::debug!("moving {} into store", item);

            let hash = item
                .move_into(&self.root)
                .context("could not move item into the store")?;

            self.associate_job_with_hash(job, &hash.to_string())
                .context("could not associate job with hash")
        }
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

/// ContentAddressedItem is responsible for hashing the outputs of a job inside
/// a workspace and (maybe) moving those outputs into the store.
#[derive(Debug)]
struct ContentAddressedItem<'job> {
    workspace: Workspace,
    job: &'job Job,
    hash: blake3::Hash,
}

impl<'job> ContentAddressedItem<'job> {
    /// Load all the outputs from a job and workspace combo, creating a hash
    /// as we go.
    fn load(job: &'job Job, workspace: Workspace) -> Result<Self> {
        let mut hasher = blake3::Hasher::new();

        for path in job.outputs.iter().sorted() {
            hasher.update(path.to_string_lossy().as_bytes());

            let mut file = File::open(&workspace.join(path)).with_context(|| {
                format!(
                    "couldn't open `{}` for hashing. Did the build produce it?",
                    path.display()
                )
            })?;

            // TODO: docs for Blake3 say that a 16 KiB buffer is the most
            // efficient (for SIMD reasons), but `std::io::copy` uses an 8KiB
            // buffer. Gonna have to do this by hand at some point to take
            // advantage of the algorithm's designed speed.
            std::io::copy(&mut file, &mut hasher).with_context(|| {
                format!("could not read `{}` to calculate hash", path.display())
            })?;
        }

        Ok(ContentAddressedItem {
            workspace,
            job,
            hash: hasher.finalize(),
        })
    }

    fn hash(&self) -> &blake3::Hash {
        &self.hash
    }

    fn final_path(&self, root: &Path) -> PathBuf {
        root.join(self.to_string())
    }

    /// Does this item exist as a path within the specified root?
    fn exists_in(&self, root: &Path) -> bool {
        self.final_path(root).exists()
    }

    /// Move this item into the store. This consumes the item, since it won't be
    /// safe to do this twice (we move files from the owned `Workspace` / passed
    /// in with `load`) Returns the only safe thing to use after calling this:
    /// the hash.
    fn move_into(self, root: &Path) -> Result<blake3::Hash> {
        let final_path = self.final_path(root);

        let temp = tempfile::Builder::new()
            .prefix(&format!("tmp-{}", self))
            .tempdir_in(&root)
            .context("couldn't create temporary directory for hashing")?;

        // We optimize disk IO based on the fact that the new temporary directory
        // is completely empty: if we keep track of the directories we create,
        // we can safely assume that any errors we see are not because the path
        // already exists. No pre-creation checks or special error handling
        // necessary!
        let mut created_dirs: HashSet<PathBuf> = HashSet::new();

        for output in self.job.outputs.iter().sorted() {
            // Before we can move the file into the store, we want to make
            // sure any parent paths exist. Luckily for us, `Path.ancestors`
            // exists. Unluckily for us, it puts stuff we don't care about on
            // either end of the iterator: at the beginning, we have a blank
            // string (it would be `/` for absolute paths, but we already
            // verified we only have relative paths when constructing the
            // `Job`.) At the end, we have the full path to the file, including
            // the filename--better not make that directory! So we have to do the
            // dance below, where we remove both ends of the (non-double-ended)
            // iterator.
            let mut ancestors: Vec<&Path> = output.ancestors().skip(1).collect();
            ancestors.pop(); // removing the full path at the end of the list

            // // the collection is now ordered `[a/b/c, a/b, a]` instead of
            // `[a, a/b, a/b/c]`, but we need it to be shortest-path-first to
            // successfully create the directories in order. Reverse!
            ancestors.reverse();

            for ancestor_path in ancestors {
                let ancestor = ancestor_path.to_path_buf();

                if created_dirs.contains(&ancestor) {
                    continue;
                }

                log::trace!(
                    "creating parent directory {} in {}",
                    &ancestor.display(),
                    temp.path().display()
                );
                std::fs::create_dir(temp.path().join(&ancestor)).with_context(|| {
                    format!(
                        "could not create parent directory `{}` for output `{}`",
                        ancestor.display(),
                        output.display(),
                    )
                })?;
                created_dirs.insert(ancestor);
            }

            // Now that we have all our parent directories, we can move the
            // file over. Note that we're *moving* this file instead of copying
            // it. We no longer need the workspace around for debugging since
            // we only move things into the store if the job succeeded, so
            // we'll be removing everything in it shortly anyway!
            log::trace!("moving `{}` into store path", &output.display());
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

        // Now that we're all done moving files over and making them read-only,
        // we can safely make all the directories read-only too.
        for dir in &created_dirs {
            Self::make_readonly(&temp.path().join(&dir)).with_context(|| {
                format!("could not make `{}` read-only in the store", dir.display(),)
            })?;
        }

        // important: at this point we need to take ownership of the tempdir so
        // that it doesn't get automatically removed when it's dropped. We've
        // so far avoided that to avoid leaving temporary directories laying
        // around in case of errors.
        std::fs::rename(temp.into_path(), &final_path)
            .context("could not move temporary collection directory into the store")?;
        Self::make_readonly(&final_path).context("could not make store path readonly")?;

        Ok(self.hash)
    }

    fn make_readonly(path: &Path) -> Result<()> {
        let mut perms = std::fs::metadata(&path)
            .context("could not get file metadata")?
            .permissions();

        perms.set_readonly(true);

        std::fs::set_permissions(&path, perms).context("could not set permissions")
    }
}

impl<'job> Display for ContentAddressedItem<'job> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.hash.fmt(f)
    }
}
