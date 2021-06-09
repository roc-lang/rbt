use notify::{raw_watcher, RecursiveMode, Watcher};
use std::sync::mpsc::channel;

pub fn run() {
    let path = "../roc"; // TODO read from CLI args, default to cwd()

    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // The notification back-end is selected based on the platform.
    let mut watcher = raw_watcher(tx).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path, RecursiveMode::Recursive).unwrap();

    loop {
        match rx.recv() {
            Ok(event) => println!("{:?}", event),
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}
