use crate::job::{self, Job};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;

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

        Ok(Store {
            root,
            inputs_to_content,
        })
    }

    pub fn for_job(&self, job: &Job) -> Option<PathBuf> {
        println!("{:#?}", job);
        None
    }
}
