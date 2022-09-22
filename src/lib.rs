#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

mod cli;
mod coordinator;
mod glue;
mod job;
mod runner;
mod store;
mod workspace;

use clap::Parser;
use std::ffi::{c_void, CStr};
use std::os::raw::c_char;

#[no_mangle]
pub(crate) unsafe extern "C" fn roc_alloc(size: usize, _alignment: u32) -> *mut c_void {
    libc::malloc(size)
}

#[no_mangle]
pub(crate) unsafe extern "C" fn roc_realloc(
    c_ptr: *mut c_void,
    new_size: usize,
    _old_size: usize,
    _alignment: u32,
) -> *mut c_void {
    libc::realloc(c_ptr, new_size)
}

#[no_mangle]
pub(crate) unsafe extern "C" fn roc_dealloc(c_ptr: *mut c_void, _alignment: u32) {
    libc::free(c_ptr)
}

#[no_mangle]
pub(crate) unsafe extern "C" fn roc_panic(c_ptr: *mut c_void, tag_id: u32) {
    match tag_id {
        0 => {
            let slice = CStr::from_ptr(c_ptr as *const c_char);
            let string = slice.to_str().unwrap();
            log::error!("Roc hit a panic: {}", string);
            std::process::exit(1);
        }
        _ => todo!(),
    }
}

#[no_mangle]
pub(crate) unsafe extern "C" fn roc_memcpy(
    dst: *mut c_void,
    src: *mut c_void,
    n: usize,
) -> *mut c_void {
    libc::memcpy(dst, src, n)
}

#[no_mangle]
pub(crate) unsafe extern "C" fn roc_memset(dst: *mut c_void, c: i32, n: usize) -> *mut c_void {
    libc::memset(dst, c, n)
}

#[no_mangle]
pub unsafe extern "C" fn roc_shm_open(name: *const i8, oflag: i32, mode: u32) -> i32 {
    libc::shm_open(name, oflag, mode)
}

#[no_mangle]
pub unsafe extern "C" fn roc_mmap(
    addr: *mut c_void,
    len: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    offset: i64,
) -> *mut c_void {
    libc::mmap(addr, len, prot, flags, fd, offset)
}

#[no_mangle]
pub unsafe extern "C" fn roc_kill(pid: i32, sig: i32) -> i32 {
    libc::kill(pid, sig)
}

#[no_mangle]
pub unsafe extern "C" fn roc_getppid() -> i32 {
    libc::getppid()
}

#[no_mangle]
pub fn rust_main() -> isize {
    println!("test got to rust_main");
    let cli = cli::Cli::parse();
    println!("parsed");

    simple_logger::SimpleLogger::new()
        .init()
        .expect("failed to initialize logger");

    if let Err(problem) = cli.run() {
        eprintln!("{:?}", problem);
        1
    } else {
        0
    }
}
