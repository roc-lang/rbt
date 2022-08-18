use crate::job::{self, Job};
use anyhow::{Context, Result};
use itertools::Itertools;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::path::{Component, Path, PathBuf};

/// This struct manages all the levels of storage that we need in order to avoid
/// doing as much work as possible. This mostly involves managing several layers
/// of caches:
#[derive(Debug)]
pub struct Store {
    root: PathBuf,

    // This is stored as JSON for now to avoid taking another dependency,
    // but it'd be good for it to be a real database (or database table)
    // eventually. SQLite or Sled or something
    inputs_to_content: HashMap<job::Id, PathBuf>,
}

impl Store {
    pub fn new(root: PathBuf) -> Result<Self> {
        let inputs_to_content = match std::fs::File::open(&root.join("inputs_to_content.json")) {
            Ok(file) => {
                let reader = std::io::BufReader::new(file);
                serde_json::from_reader(reader)
                    .context("could not deserialize mapping from inputs to content")?
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => HashMap::default(),
            Err(err) => return Err(err).context("could not open mapping from inputs to content"),
        };

        if !root.exists() {
            std::fs::create_dir_all(&root).context("could not create specified root")?;
        }

        Ok(Store {
            root,
            inputs_to_content,
        })
    }

    pub fn for_job(&self, job: &Job) -> Option<PathBuf> {
        println!("{:#?}", job);
        None
    }

    pub fn store_from_workspace(&self, job: &Job, workspace: &Path) -> Result<()> {
        let mut hasher = blake3::Hasher::new();
        let temp = tempfile::Builder::new()
            .prefix(&format!("rbt-job-{}", job.id))
            .tempdir()
            .context("couldn't create temporary directory for hashing")?;

        let mut output_dirs: HashSet<PathBuf> = HashSet::new();

        for output in job.outputs.iter().sorted() {
            let source = PathBuf::from(output.as_str());

            ///////////////////////////////////////////////////
            // Step 1: Validate that the path we got is safe //
            ///////////////////////////////////////////////////
            for component in source.components() {
                match component {
                    Component::Prefix(_) | Component::RootDir =>  anyhow::bail!(
                        "Got `{}` as an output, but absolute paths are not allowed as outputs. Remove the absolute prefix to fix this!",
                        source.display(),
                    ),

                    Component::ParentDir => anyhow::bail!(
                        "Got `{}` as an output, but relative paths containing `..` are not allowed as inputs. Remove the `..` to fix this!",
                        source.display(),
                    ),

                    Component::CurDir | Component::Normal(_) => (),
                };
            }

            ////////////////////////////////////////////////////////////////
            // Step 2: add the file content to the content-addressed hash //
            ////////////////////////////////////////////////////////////////
            let mut file = File::open(&workspace.join(output.as_str())).with_context(|| {
                format!(
                    "couldn't open `{}` for hashing. Did the build produce it?",
                    output
                )
            })?;

            // TODO: docs for Blake3 say that a 16 KiB buffer is the most
            // efficient (for SIMD reasons), but `std::io::copy` uses an 8KiB
            // buffer. Gonna have to do this by hand at some point to take
            // advantage of the algorithm's designed speed.
            std::io::copy(&mut file, &mut hasher).with_context(|| {
                format!("could not read `{}` to calculate hash", source.display())
            })?;

            ///////////////////////////////////////////////
            // Step 3: make sure any parent paths exist  //
            ///////////////////////////////////////////////
            let mut ancestors: Vec<&Path> = source.ancestors().skip(1).collect();
            ancestors.pop(); // we've made sure this is relative, so the first item is ""
            ancestors.reverse(); // go `[a, a/b, a/b/c]` instead of `[a/b/c, a/b, a]`

            for ancestor_path in ancestors {
                let ancestor = ancestor_path.to_path_buf();

                if output_dirs.contains(&ancestor) {
                    continue;
                }

                std::fs::create_dir(temp.path().join(&ancestor)).with_context(|| {
                    format!(
                        "could not create ancestor `{}` for output `{}`",
                        ancestor.display(),
                        source.display(),
                    )
                })?;
                output_dirs.insert(ancestor);
            }

            //////////////////////////////
            // Step 4: collect the file //
            //////////////////////////////
            let out = temp.path().join(&source);
            std::fs::rename(workspace.join(&source), &out).with_context(|| {
                format!(
                    "could not move `{}` from workspace to store",
                    source.display()
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

        let final_location = self.root.join(hasher.finalize().to_hex().to_string());

        // important: at this point we need to take ownership of the tempdir so
        // that it doesn't get automatically removed when it's dropped. We've
        // so far avoided that to avoid leaving temporary directories laying
        // around in case of errors.
        std::fs::rename(temp.into_path(), &final_location)
            .context("could not move temporary collection dir into the store")?;
        Self::make_readonly(&final_location).context("could not make store path readonly")
    }

    fn make_readonly(path: &Path) -> Result<()> {
        let mut perms = std::fs::metadata(&path)
            .context("could not get file metadata")?
            .permissions();

        perms.set_readonly(true);

        std::fs::set_permissions(&path, perms).context("could not set permissions")
    }
}
