#![allow(non_snake_case)]

mod job;

use core::mem::MaybeUninit;
use roc_std::{RocList, RocStr};
use std::ffi::{c_void, CStr};
use std::fmt;
use std::os::raw::c_char;

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
    args: RocList<RocStr>,
    tool: RbtTool,
}

#[repr(C)]
struct RbtTool {
    payload: RbtToolPayload,
    tag: i64,
}

impl fmt::Debug for RbtTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.tag {
            0 => f
                .debug_struct("from_job")
                .field("name", unsafe { &self.payload.from_job })
                .finish(),
            1 => f
                .debug_struct("system_tool")
                .field("name", unsafe { &self.payload.system_tool })
                .finish(),
            _ => panic!(
                "I don't know what payload this tag ({:}) should be associated with!",
                self.tag
            ),
        }
    }
}

#[repr(C)]
union RbtToolPayload {
    system_tool: core::mem::ManuallyDrop<RocStr>,
    // fromJob: (Job, RocStr)
    from_job: core::mem::ManuallyDrop<RocStr>,
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
pub unsafe extern "C" fn roc_panic(c_ptr: *mut c_void, tag_id: u32) {
    match tag_id {
        0 => {
            let slice = CStr::from_ptr(c_ptr as *const c_char);
            let string = slice.to_str().unwrap();
            eprintln!("Roc hit a panic: {}", string);
            std::process::exit(1);
        }
        _ => todo!(),
    }
}

#[no_mangle]
pub fn rust_main() -> isize {
    println!("about to rbt_uninit");
    let mut rbt_uninit: MaybeUninit<Rbt> = MaybeUninit::uninit();

    unsafe {
        println!("roc_init");
        roc_init(rbt_uninit.as_mut_ptr());

        println!("rbt_uninit");
        let rbt = rbt_uninit.assume_init();

        // println!(
        //     "{:?}",
        //     std::mem::transmute::<RbtTool, [u8; 24]>(rbt.default.command.tool)
        // );
        println!("{:#?}", rbt);

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
