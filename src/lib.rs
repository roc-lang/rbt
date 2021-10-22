#![allow(non_snake_case)]

mod job;

use core::ffi::c_void;
use core::mem::MaybeUninit;
use roc_std::{RocList, RocStr};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

#[repr(C)]
struct RocJob {
    arguments: RocList<RocStr>,
    command: RocStr,
    inputs: RocList<RocStr>,
    outputs: RocList<RocStr>,
    working_directory: RocStr,
}

extern "C" {
    #[link_name = "roc__mainForHost_1_exposed"]
    fn roc_main(output: *mut RocJob) -> ();
}

#[no_mangle]
pub unsafe fn roc_alloc(size: usize, _alignment: u32) -> *mut c_void {
    return libc::malloc(size);
}

#[no_mangle]
pub unsafe fn roc_realloc(
    c_ptr: *mut c_void,
    new_size: usize,
    _old_size: usize,
    _alignment: u32,
) -> *mut c_void {
    return libc::realloc(c_ptr, new_size);
}

#[no_mangle]
pub unsafe fn roc_dealloc(c_ptr: *mut c_void, _alignment: u32) {
    return libc::free(c_ptr);
}

#[no_mangle]
pub fn rust_main() -> isize {
    let mut job: MaybeUninit<RocJob> = MaybeUninit::uninit();

    unsafe {
        roc_main(job.as_mut_ptr());

        let roc_job = job.assume_init();

        let args: Vec<String> = roc_job
            .arguments
            .as_slice()
            .iter()
            .map(|file| file.as_str().to_string())
            .collect();

        let inputs: Vec<PathBuf> = roc_job
            .inputs
            .as_slice()
            .iter()
            .map(|path| PathBuf::from(path.as_str()))
            .collect();

        let outputs: Vec<PathBuf> = roc_job
            .outputs
            .as_slice()
            .iter()
            .map(|path| PathBuf::from(path.as_str()))
            .collect();

        let job = job::Job {
            // TODO: these should eventually be RocStrs, and Job should
            // just accept and convert those accordingly.
            command: roc_job.command.as_str().to_string(),
            arguments: args.clone(),
            environment: HashMap::default(),
            working_directory: PathBuf::from(roc_job.working_directory.as_str()),
            inputs: inputs,
            outputs: outputs,
        };
        job.run().expect("TODO better platform error handling");
    }

    println!("All done!");

    // Exit code
    0
}

#[test]
fn test_examples() {
    let status = Command::new("roc")
        .args(&["examples/ReadSelf.roc"])
        .status()
        .unwrap();
    assert_eq!(status.success(), true);
}