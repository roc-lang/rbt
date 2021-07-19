use notify::{raw_watcher, RecursiveMode, Watcher};
use std::sync::mpsc::channel;

use crate::cache::Cache;
use crate::deps::Deps;
use crate::job;
use anyhow::Result;
use std::collections::HashMap;

pub fn run() -> Result<()> {
    {
        use std::path::Path;

        let mut deps = Deps::default();
        let mut cache = Cache::new(Path::new("roc-stuff"))?;

        deps.add(Path::new("blah"), &|_| &[]);
        deps.find_changed(&mut cache)?;
    }

    let path = "../roc"; // TODO read from CLI args, default to cwd()

    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // The notification back-end is selected based on the platform.
    let mut watcher = raw_watcher(tx).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path, RecursiveMode::Recursive).unwrap();

    // Run a random command to get clippy to let us keep Job.run (for now.)
    let job = job::Job {
        command: "echo".to_string(),
        arguments: vec![],
        environment: HashMap::default(),
        working_directory: std::env::current_dir()?,
        inputs: vec![],
        outputs: vec![],
    };
    job.run().unwrap();

    loop {
        match rx.recv() {
            Ok(event) => println!("{:?}", event),
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}
