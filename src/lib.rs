#![allow(non_snake_case)]

mod job;

use core::ffi::c_void;
use core::mem::MaybeUninit;
use roc_std::{RocList, RocStr};
use std::fmt;

#[derive(Debug)]
#[repr(C)]
struct Rbt {
    default: RbtJob,
}

#[derive(Debug)]
#[repr(C)]
struct RbtJob {
    command: RbtCommand,
    inputs: RocList<RbtJob>,
    input_files: RocList<RocStr>,
    outputs: RocList<RocStr>,
}

#[derive(Debug)]
#[repr(C)]
struct RbtCommand {
    tool: RbtTool,
    args: RocList<RocStr>,
}

#[derive(Debug)]
#[repr(C)]
struct RbtTool {
    payload: RbtToolPayload,
    tag: i64,
}

#[repr(C)]
union RbtToolPayload {
    system_tool: core::mem::ManuallyDrop<RocStr>,
    // fromJob: (Job, RocStr)
    from_job: core::mem::ManuallyDrop<RocStr>,
}

impl fmt::Debug for RbtToolPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("lol")
    }
}

extern "C" {
    #[link_name = "roc__initForHost_1_exposed"]
    fn roc_init(output: *mut Rbt) -> ();
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
    let mut rbt_uninit: MaybeUninit<Rbt> = MaybeUninit::uninit();

    unsafe {
        roc_init(rbt_uninit.as_mut_ptr());

        let rbt = rbt_uninit.assume_init();

        println!("{:?}", rbt);

        // let args: Vec<String> = roc_job
        //     .arguments
        //     .as_slice()
        //     .iter()
        //     .map(|file| file.as_str().to_string())
        //     .collect();

        // let inputs: Vec<PathBuf> = roc_job
        //     .inputs
        //     .as_slice()
        //     .iter()
        //     .map(|path| PathBuf::from(path.as_str()))
        //     .collect();

        // let outputs: Vec<PathBuf> = roc_job
        //     .outputs
        //     .as_slice()
        //     .iter()
        //     .map(|path| PathBuf::from(path.as_str()))
        //     .collect();

        // let job = job::Job {
        //     // TODO: these should eventually be RocStrs, and Job should
        //     // just accept and convert those accordingly.
        //     command: roc_job.command.as_str().to_string(),
        //     arguments: args.clone(),
        //     environment: HashMap::default(),
        //     working_directory: PathBuf::from(roc_job.working_directory.as_str()),
        //     inputs: inputs,
        //     outputs: outputs,
        // };
        // job.run().expect("TODO better platform error handling");
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
