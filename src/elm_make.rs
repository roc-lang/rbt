use roc_std::{RocList, RocStr};
use std::iter;
use std::process::Command;

pub fn build(source_files: RocList<RocStr>) {
    let standard_args = iter::once("make");

    let mut file_args = source_files
        .as_slice()
        .iter()
        .map(|file| unsafe { file.as_str() });

    let args: Vec<&str> = standard_args.chain(file_args).collect();

    let status = Command::new("elm")
        .args(args)
        .status()
        .expect("couldn't launch `elm`");
    assert!(status.success());
}
