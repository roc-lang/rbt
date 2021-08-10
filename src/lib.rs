#![allow(non_snake_case)]

mod job;

use core::ffi::c_void;
use core::mem::MaybeUninit;
use roc_std::{RocCallResult, RocList, RocStr};
use std::collections::HashMap;
use std::path::PathBuf;

extern "C" {
    #[link_name = "roc__mainForHost_1_exposed"]
    fn roc_main(output: *mut RocCallResult<RocList<RocStr>>) -> ();
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
    let mut call_result: MaybeUninit<RocCallResult<RocList<RocStr>>> = MaybeUninit::uninit();

    unsafe {
        roc_main(call_result.as_mut_ptr());

        let output = call_result.assume_init();

        match output.into() {
            Ok(source_files) => {
                let args: Vec<String> = source_files
                    .as_slice()
                    .iter()
                    // TODO: these should eventually be RocStrs, and Job should
                    // just accept and convert those accordingly.
                    .map(|file| file.as_str().to_string())
                    .collect();

                let job = job::Job {
                    command: String::from("cat"),
                    arguments: args.clone(),
                    environment: HashMap::default(),
                    working_directory: PathBuf::from("."),
                    inputs: args.iter().map(|arg| PathBuf::from(arg)).collect(),
                    outputs: Vec::new(),
                };
                job.run().expect("TODO better platform error handling");
            }
            Err(msg) => {
                panic!("Roc failed with message: {}", msg);
            }
        }
    }

    // Exit code
    0
}
