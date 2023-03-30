use core::alloc::Layout;

use crate::{iostream::STDIN, memory, scheduler};
use fs_rs::fs;

pub const EXEC: u64 = 0x3b;
pub const EXIT: u64 = 0x3c;
pub const READ: u64 = 0;
pub const MALLOC: u64 = 0x9;
pub const FREE: u64 = 0xb;
pub const CREATE_FILE: u64 = 0x2;
pub const REMOVE_FILE: u64 = 0x57;
// TODO read, write, ftruncate, read_dir

const STDIN_DESCRIPTOR: i32 = 0;
const STDOUT_DESCRIPTOR: i32 = 1;
const STDERR_DESCRIPTOR: i32 = 2;
const RESERVED_FILE_DESCRIPTORS: i32 = 3;
const ALIGNMENT: usize = 16;

/// Create a file in the file system.
///
/// # Arguments
/// - `path` - Path to the file.
/// - `path_len` - Length of the path.
/// - `directory` - Whether the new file should be a directory.
///
/// # Returns
/// The file descriptor of the new file if the operation was successful, -1 otherwise.
pub unsafe fn create_file(path: *mut u8, directory: bool) -> i32 {
    let p = scheduler::get_running_process().as_ref().unwrap();
    let name_str;

    if let Some(name) = super::get_user_str(p, path) {
        name_str = name;
    } else {
        return -1;
    }

    if fs::create_file(name_str, directory, Some(p.cwd())).is_ok() {
        // UNWRAP: The file creation was successful.
        fs::get_file_id(name_str, Some(p.cwd())).unwrap() as i32 + RESERVED_FILE_DESCRIPTORS
    } else {
        -1
    }
}

pub unsafe fn exit(_status: i32) -> i64 {
    crate::scheduler::terminator::add_to_queue(core::ptr::read(
        scheduler::get_running_process().as_mut().unwrap(),
    ));
    core::ptr::write(scheduler::get_running_process(), None);

    return 0;
}

/// Remove a file from the file system, or remove a directory that must be empty.
///
/// # Arguments
/// - `path` - Path to the file.
/// - `path_len` - Length of the path.
///
/// # Returns
/// 0 if the operation was successful, -1 otherwise.
pub unsafe fn remove_file(path: *mut u8) -> i64 {
    let p = scheduler::get_running_process().as_ref().unwrap();
    let name_str;

    if let Some(name) = super::get_user_str(p, path) {
        name_str = name;
    } else {
        return -1;
    }

    if fs::remove_file(name_str, Some(p.cwd())).is_ok() {
        0
    } else {
        -1
    }
}

/// Read bytes from a file descriptor.
///
/// # Arguments
/// - `fd` - The file descriptor to read from.
/// - `user_buffer` - The buffer to write into.
/// - `count` - The number of bytes to read.
/// - `offset` - The offset in the file to start reading from, ignored for `stdin`.
///
/// # Returns
/// 0 if the operation was successful, -1 otherwise.
pub unsafe fn read(fd: i32, user_buffer: *mut u8, count: usize, offset: usize) -> i64 {
    let p = scheduler::get_running_process().as_ref().unwrap();
    let buf;
    let file_id = (fd - RESERVED_FILE_DESCRIPTORS) as usize;

    if let Some(buffer) = super::get_user_buffer(p, user_buffer, count) {
        buf = buffer;
    } else {
        return -1;
    }

    if fd < RESERVED_FILE_DESCRIPTORS {
        match fd {
            STDIN_DESCRIPTOR => STDIN.read(buf),
            STDOUT_DESCRIPTOR => return 0, // STDOUT still not implemented
            STDERR_DESCRIPTOR => return 0, // STDERR still not implemented
            _ => 0,
        };
    }

    match fs::read(file_id, buf, offset) {
        Some(b) => {
            if fs::is_dir(file_id) {
                return -1;
            }
            b as i64
        }
        None => -1,
    }
}

/// function that execute a process
///
/// # Arguments
/// - `name` - pointer to i8 (the equivalent to c char) and execute the file that have this name
///
/// # Returns
/// 0 if the operation was successful, -1 otherwise
pub unsafe fn exec(name: *const i8) -> i64 {
    let mut len: usize = 0;
    while *(name.add(len)) != 0 {
        len += 1;
        if len > fs::FILE_NAME_LEN {
            return -1;
        }
    }

    let bytes: &[u8] = core::slice::from_raw_parts(name as *mut u8, len);
    let file_name = if let Ok(v) = core::str::from_utf8(bytes) {
        v
    } else {
        return -1;
    };

    let id = if let Some(id) = fs::get_file_id(file_name, None) {
        id
    } else {
        return -1;
    };

    if let Ok(proc) = scheduler::Process::new_user_process(
        id as u64,
        scheduler::get_running_process().as_ref().unwrap().cwd(),
    ) {
        scheduler::add_to_the_queue(proc);
    } else {
        return -1;
    };

    0
}

/// Allocate memory for a userspace program.
///
/// # Arguments
/// - `size` - The size of the allocation.
///
/// # Returns
/// A pointer to the allocation or null on failure.
pub unsafe fn malloc(size: usize) -> *mut u8 {
    let allocator = scheduler::get_running_process()
        .as_mut()
        .unwrap()
        .allocator();
    let layout = Layout::from_size_align(size, ALIGNMENT);
    let mut allocation = core::ptr::null_mut();

    if let Ok(layout) = layout {
        memory::load_tables_to_cr3(allocator.get_page_table());
        allocation = allocator.global_alloc(layout);
        memory::load_tables_to_cr3(memory::PAGE_TABLE);
    }

    allocation
}

/// Deallocate an allocation that was allocated with `malloc`.
///
/// # Arguments
/// - `ptr` - The pointer to the allocation that was returned from `malloc`.
pub unsafe fn free(ptr: *mut u8) -> i64 {
    scheduler::get_running_process()
        .as_mut()
        .unwrap()
        .allocator()
        .global_dealloc(ptr, Layout::from_size_align(0, 1).unwrap());

    0
}
