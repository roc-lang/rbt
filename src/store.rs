use crate::job::{self, Job};
use crate::workspace::Workspace;
use anyhow::{Context, Result};
use itertools::Itertools;
use std::collections::HashSet;
use std::fmt::{self, Display};
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::AsyncReadExt;

/// Store is responsible for managing a content-addressed store below some path
/// and managing the associations between input job hashes and those paths.
#[derive(Debug)]
pub struct Store {
    root: PathBuf,
    db: sled::Tree,
}

impl Store {
    pub fn new(db: sled::Tree, root: PathBuf) -> Result<Self> {
        if !root.exists() {
            log::info!("creating store root at {}", &root.display());
            std::fs::create_dir_all(&root).context("could not create specified root")?;
        }

        Ok(Store { root, db })
    }

    pub fn item_for_job(&self, key: &job::Key<job::Final>) -> Result<Option<Item>> {
        match self
            .db
            .get(key.to_db_key())
            .context("could not read from store DB")?
        {
            None => Ok(None),
            Some(hash) => Item::from_hex(&self.root, hash.as_ref()).map(Some),
        }
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
    ///     know we need to store the output.
    ///  3. All the paths in the Job's `output` field have been sanitized (that
    ///     is, they don't include any paths leading to the root or other
    ///     drives, or contain `..` elements that would take the path out of
    ///     the workspace root.)
    pub async fn store_from_workspace(
        &mut self,
        key: job::Key<job::Final>,
        job: &Job,
        workspace: Workspace,
    ) -> Result<Item> {
        let item_builder = ItemBuilder::load(&self.root, job, workspace)
            .await
            .context("could get content addressed path from job")?;

        let item = item_builder
            .move_into_checked(&self.root)
            .await
            .context("could not move item into the store")?;

        self.associate_job_with_hash(key, &item.to_string())
            .context("could not associate job with hash")?;

        Ok(item)
    }

    fn associate_job_with_hash(&mut self, key: job::Key<job::Final>, hash: &str) -> Result<String> {
        self.db
            .insert(key.to_db_key(), hash)
            .context("failed to write job and content-hash pair")?;

        Ok(hash.to_string())
    }
}

/// ContentAddressedItem is responsible for hashing the outputs of a job inside
/// a workspace and (maybe) moving those outputs into the store.
#[derive(Debug)]
struct ItemBuilder<'job> {
    workspace: Workspace,
    job: &'job Job,
    item: Item,
}

impl<'job> ItemBuilder<'job> {
    /// Load all the outputs from a job and workspace combo, creating a hash
    /// as we go.
    async fn load(root: &Path, job: &'job Job, workspace: Workspace) -> Result<ItemBuilder<'job>> {
        let mut hasher = blake3::Hasher::new();

        for path in job.outputs.iter().sorted() {
            match path.to_str() {
                Some(str) => hasher.update(str.as_bytes()),
                None => anyhow::bail!("got a non-unicode path `{}`, but Roc should never have produced a Str with invalid unicode.", path.display()),
            };

            let mut file = File::open(&workspace.join(path)).await.with_context(|| {
                format!(
                    "couldn't open `{}` for hashing. Did the build produce it?",
                    path.display()
                )
            })?;

            // Blake3 is designed to take advantage of SIMD instructions when
            // buffer size is 16KiB or more
            let mut buffer = [0; 16 * 1024];
            loop {
                let bytes = file.read(&mut buffer).await.with_context(|| {
                    format!("could not read `{}` to calculate hash", path.display())
                })?;
                if bytes == 0 {
                    break;
                }
                hasher.update(&buffer[0..bytes]);
            }
        }

        Ok(Self {
            workspace,
            job,
            item: Item::from_hash(root, hasher.finalize()),
        })
    }

    // like `move_into`, but checks that the store path exists first
    async fn move_into_checked(self, root: &Path) -> Result<Item> {
        if self.item.exists() {
            log::debug!("we have already stored {}, so I'm skipping the move!", self,);

            Ok(self.item)
        } else {
            log::debug!("moving {} into store", self);

            self.move_into(root)
                .await
                .context("could not move item into the store")
        }
    }

    /// Move this item into the store. This consumes the item, since it won't be
    /// safe to do this twice (we move files from the owned `Workspace` / passed
    /// in with `load`) Returns the only safe thing to use after calling this:
    /// the hash.
    async fn move_into(self, root: &Path) -> Result<Item> {
        let final_path = self.item.path();

        let temp = root.join(format!("tmp-{}", rand::random::<u64>()));
        fs::create_dir(&temp)
            .await
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

            // the collection is now ordered `[a/b/c, a/b, a]` instead of
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
                    &temp.display()
                );
                fs::create_dir(temp.join(&ancestor))
                    .await
                    .with_context(|| {
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
            let out = temp.join(output);
            fs::rename(self.workspace.join(output), &out)
                .await
                .with_context(|| {
                    format!(
                        "could not move `{}` from workspace to store",
                        output.display()
                    )
                })?;

            Self::make_readonly(&out).await.with_context(|| {
                format!(
                    "could not make `{}` read-only after moving into store",
                    out.display()
                )
            })?;
        }

        // Now that we're all done moving files over and making them read-only,
        // we can safely make all the directories read-only too.
        for dir in &created_dirs {
            Self::make_readonly(&temp.join(dir))
                .await
                .with_context(|| {
                    format!("could not make `{}` read-only in the store", dir.display(),)
                })?;
        }

        // important: at this point we need to take ownership of the tempdir so
        // that it doesn't get automatically removed when it's dropped. We've
        // so far avoided that to avoid leaving temporary directories laying
        // around in case of errors.
        fs::rename(temp, &final_path)
            .await
            .context("could not move temporary collection directory into the store")?;
        Self::make_readonly(final_path)
            .await
            .context("could not make store path readonly")?;

        Ok(self.item)
    }

    async fn make_readonly(path: &Path) -> Result<()> {
        let mut perms = fs::metadata(&path)
            .await
            .context("could not get file metadata")?
            .permissions();

        perms.set_readonly(true);

        fs::set_permissions(&path, perms)
            .await
            .context("could not set permissions")
    }
}

impl<'job> Display for ItemBuilder<'job> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.item.fmt(f)
    }
}

#[derive(Debug)]
pub struct Item {
    hash: blake3::Hash,
    path: PathBuf,
}

impl Item {
    fn from_hash(root: &Path, hash: blake3::Hash) -> Self {
        Item {
            hash,
            path: root.join(hash.to_hex().to_string()),
        }
    }

    fn from_hex(root: &Path, hex: impl AsRef<[u8]>) -> Result<Self> {
        let hash =
            blake3::Hash::from_hex(hex).context("could not load a blake3 hash from hex value")?;

        Ok(Self::from_hash(root, hash))
    }

    pub fn hash(&self) -> blake3::Hash {
        self.hash
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.hash.fmt(f)
    }
}

impl std::ops::Deref for Item {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}
