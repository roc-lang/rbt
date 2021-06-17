use notify::{raw_watcher, RecursiveMode, Watcher};
use std::sync::mpsc::channel;

use crate::cache::Cache;
use crate::deps::Deps;
use crate::job;
use std::collections::HashMap;
use std::io;

pub fn run() -> io::Result<()> {
    // TODO this is just so we don't get unused warnings.
    {
        use std::path::Path;

        let mut deps = Deps::default();
        let mut cache = Cache::default();

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
        inputs: vec![],
    };
    job.run().unwrap();

    loop {
        match rx.recv() {
            Ok(event) => println!("{:?}", event),
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}
