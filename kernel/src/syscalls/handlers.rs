use core::alloc::{GlobalAlloc, Layout};

use crate::{
    iostream::STDIN,
    memory::{self, allocator},
    scheduler,
};
use alloc::{string::ToString, vec::Vec};
use fs_rs::fs::{self, DirEntry};

pub const READ: u64 = 0x0;
pub const WRITE: u64 = 0x1;
pub const OPEN: u64 = 0x2;
pub const FSTAT: u64 = 0x5;
pub const WAITPID: u64 = 0x7;
pub const MALLOC: u64 = 0x9;
pub const CALLOC: u64 = 0xa;
pub const FREE: u64 = 0xb;
pub const REALLOC: u64 = 0xc;
pub const SCHED_YIELD: u64 = 0x18;
pub const EXEC: u64 = 0x3b;
pub const EXIT: u64 = 0x3c;
pub const GET_CURRENT_DIR_NAME: u64 = 0x4f;
pub const CHDIR: u64 = 0x50;
pub const CREAT: u64 = 0x55;
pub const REMOVE_FILE: u64 = 0x57;
pub const READ_DIR: u64 = 0x59;
pub const TRUNCATE: u64 = 0x4c;
pub const FTRUNCATE: u64 = 0x4d;

const STDIN_DESCRIPTOR: i32 = 0;
const STDOUT_DESCRIPTOR: i32 = 1;
const STDERR_DESCRIPTOR: i32 = 2;
const RESERVED_FILE_DESCRIPTORS: i32 = 3;

#[allow(unused)]
pub struct Stat {
    size: u64,
    directory: bool,
}

/// Get the current working directory.
///
/// # Returns
/// On success, a string containing the current working directory
/// that has been allocated with `malloc` will be returned.
/// It is the user's responsibility to free the buffer with `free`.
/// On failure, null is returned.
pub unsafe fn get_current_dir_name() -> *mut u8 {
    let path = scheduler::get_running_process()
        .as_ref()
        .unwrap()
        .cwd_path();
    let buffer = malloc(path.len() + 1);

    if !buffer.is_null() {
        core::ptr::copy_nonoverlapping(path.as_ptr(), buffer, path.len());
        // Add null terminator.
        *buffer.add(path.len()) = 0;
    }

    buffer
}

/// Change the current working directory.
///
/// # Arguments
/// - `path` - Path to the new working directory.
///
/// # Returns
/// 0 if the operation was successful or -1 on failure.
/// Possible failures:
/// - `path` is invalid.
/// - `path` does not exist.
/// - `path` is not a directory.
pub unsafe fn chdir(path: *const u8) -> i64 {
    let p = scheduler::get_running_process().as_mut().unwrap();
    let file_id;
    let path_str;
    let combined_path;
    let absolute_path;

    if let Some(path) = super::get_user_str(p, path) {
        path_str = path;
    } else {
        return -1;
    }
    if let Some(id) = fs::get_file_id(path_str, Some(p.cwd())) {
        file_id = id;
    } else {
        return -1;
    }

    combined_path = if p.cwd_path().ends_with('/') {
        p.cwd_path().to_string() + path_str
    } else {
        p.cwd_path().to_string() + "/" + path_str
    };
    if fs::is_dir(file_id).unwrap_or(false) {
        absolute_path = if path_str.starts_with('/') {
            super::get_absolute_path(&path_str)
        } else {
            super::get_absolute_path(&combined_path)
        };
        p.set_cwd(&absolute_path);

        0
    } else {
        -1
    }
}

/// Create a file in the file system.
///
/// # Arguments
/// - `path` - Path to the file.
/// - `path_len` - Length of the path.
/// - `directory` - Whether the new file should be a directory.
///
/// # Returns
/// The file descriptor of the new file if the operation was successful, -1 otherwise.
pub unsafe fn creat(path: *const u8, directory: bool) -> i32 {
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

/// Terminate the calling process.
///
/// # Arguments
/// - `status` - The exit code of the process.
pub unsafe fn exit(status: i32) -> i64 {
    let p = core::mem::replace(scheduler::get_running_process(), None).unwrap();

    scheduler::stop_waiting_for(&p, status);
    scheduler::terminator::add_to_queue(p);

    0
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
/// - `buf` - The buffer to write into.
/// - `count` - The number of bytes to read.
/// - `offset` - The offset in the file to start reading from, ignored for `stdin`.
///
/// # Returns
/// The amount of bytes read or -1 on failure.
pub unsafe fn read(fd: i32, buf: *mut u8, count: usize, offset: usize) -> i64 {
    let p = scheduler::get_running_process().as_ref().unwrap();
    let buffer;
    let file_id;

    if let Some(buf) = super::get_user_buffer_mut(p, buf, count) {
        buffer = buf;
    } else {
        return -1;
    }
    if fd < 0 {
        return -1;
    }

    match fd {
        STDIN_DESCRIPTOR => STDIN.read(buffer) as i64,
        STDOUT_DESCRIPTOR => -1, // STDOUT still not implemented
        STDERR_DESCRIPTOR => -1, // STDERR still not implemented
        _ => {
            file_id = (fd - RESERVED_FILE_DESCRIPTORS) as usize;
            if fs::is_dir(file_id).unwrap_or(true) {
                -1
            } else {
                match fs::read(file_id, buffer, offset) {
                    Some(b) => b as i64,
                    None => -1,
                }
            }
        }
    }
}

/// Write bytes to a file descriptor.
///
/// # Arguments
/// - `fd` - The file descriptor to write to.
/// - `buf` - A buffer containing the data to be written.
/// - `offset` - The offset where the data will be written in the file,
/// this is ignored for `stdout`.
/// If the offset is at the end of the file or the data after it is written overflows the file's
/// length the file will be extended.
/// If the offset is beyond the file's size the file will be extended and a "hole" will be
/// created in the file. Reading from the hole will return null bytes.
///
/// # Returns
/// 0 if the operation was successful, -1 otherwise.
pub unsafe fn write(fd: i32, buf: *const u8, count: usize, offset: usize) -> i64 {
    let p = scheduler::get_running_process().as_ref().unwrap();
    let buffer;
    let file_id;

    if let Some(buf) = super::get_user_buffer(p, buf, count) {
        buffer = buf;
    } else {
        return -1;
    }
    if fd < 0 {
        return -1;
    }

    match fd {
        STDIN_DESCRIPTOR => -1, // STDIN still not implemented
        STDOUT_DESCRIPTOR => {
            if let Ok(string) = core::str::from_utf8(buffer) {
                memory::load_tables_to_cr3(memory::get_page_table());
                crate::print!("{}", string);

                0
            } else {
                -1
            }
        }
        STDERR_DESCRIPTOR => -1, // STDERR still not implemented
        _ => {
            file_id = (fd - RESERVED_FILE_DESCRIPTORS) as usize;
            if fs::is_dir(file_id).unwrap_or(true) {
                -1
            } else {
                if fs::write(file_id, buffer, offset).is_ok() {
                    0
                } else {
                    -1
                }
            }
        }
    }
}

/// Get a file descriptor for a file.
///
/// # Arguments
/// - `pathname` - Path to the file.
///
/// # Returns
/// The file descriptor for the file on success or -1 otherwise.
pub unsafe fn open(pathname: *const u8) -> i32 {
    let p = scheduler::get_running_process().as_ref().unwrap();
    let path_str;

    if let Some(path) = super::get_user_str(p, pathname) {
        path_str = path;
    } else {
        return -1;
    }

    if let Some(id) = fs::get_file_id(path_str, Some(p.cwd())) {
        id as i32 + RESERVED_FILE_DESCRIPTORS
    } else {
        -1
    }
}

/// Get information about a file.
///
/// # Arguments
/// - `fd` - The file descriptor of that file.
/// - `statbuf` - A buffer to the `Stat` struct that will contain the information about the file.
///
/// # Returns
/// 0 if the file exists and -1 if it doesn't or if `fd` is negative.
pub unsafe fn fstat(fd: i32, statbuf: *mut Stat) -> i64 {
    if fd < 0 {
        return -1;
    }

    if let Some(size) = fs::get_file_size(fd as usize) {
        *statbuf = Stat {
            size: size as u64,
            // UNWRAP: We already checked that the file exists.
            directory: fs::is_dir(fd as usize).unwrap(),
        };

        0
    } else {
        -1
    }
}

/// Awaits the calling process until a specific process terminates.
///
/// # Arguments
/// - `pid` - The process ID of the process to wait for.
/// Must be a non-negative number.
/// - `wstatus` - A buffer to write the process' exit code into.
///
/// # Returns
/// 0 on sucess or -1 on error.
/// Possible errors:
/// - `pid` is negative.
/// - The process specified by `pid` does not exist.
/// - The process specified by `pid` has already finished its execution.
pub unsafe fn waitpid(pid: i64, wstatus: *mut i32) -> i64 {
    let p;

    if pid < 0 {
        return -1;
    }

    // Write to `wstatus` to avoid any errors with it later.
    *wstatus = 0;
    if scheduler::search_process(pid) {
        p = core::mem::replace(scheduler::get_running_process(), None).unwrap();
        scheduler::wait_for(pid, p, wstatus);

        0
    } else {
        -1
    }
}

/// Change the length of a file to a specific length.
/// If the file has been set to a greater length, reading the extra data will return null bytes
/// until the data is being written.
/// If the file has been set to a smaller length, the extra data will be lost.
///
/// # Arguments
/// - `fd` - The file descriptor of the file.
/// - `length` - The required size.
///
/// # Returns
/// 0 if the operation was successful, -1 otherwise.
pub unsafe fn ftruncate(fd: i32, length: u64) -> i64 {
    let file_id;

    if fd < 0 {
        return -1;
    }

    if fd >= RESERVED_FILE_DESCRIPTORS {
        file_id = (fd - RESERVED_FILE_DESCRIPTORS) as usize;
        if fs::is_dir(file_id).unwrap_or(true) {
            -1
        } else {
            if fs::set_len(fd as usize, length as usize).is_ok() {
                0
            } else {
                -1
            }
        }
    } else {
        -1
    }
}

/// Change the length of a file to a specific length.
/// If the file has been set to a greater length, reading the extra data will return null bytes
/// until the data is being written.
/// If the file has been set to a smaller length, the extra data will be lost.
///
/// # Arguments
/// - `path` - Path to the file.
/// - `length` - The required size.
///
/// # Returns
/// 0 if the operation was successful, -1 otherwise.
pub unsafe fn truncate(path: *const u8, length: u64) -> i64 {
    let p = scheduler::get_running_process().as_ref().unwrap();
    let path_str;

    if let Some(string) = super::get_user_str(p, path) {
        path_str = string;
    } else {
        return -1;
    }

    if let Some(file) = fs::get_file_id(path_str, Some(p.cwd())) {
        ftruncate(file as i32 + RESERVED_FILE_DESCRIPTORS, length)
    } else {
        -1
    }
}

/// Read a directory entry.
///
/// # Arguments
/// - `fd` - The file descriptor of the directory.
/// - `offset` - The offset **in files** inside the directory to read from.
/// - `dirp` - A buffer to write the data into.
///
/// # Returns
/// 0 on success, -1 on failure.
/// Possible failures:
/// - `fd` is negative or invalid.
/// - `fd` is a directory.
pub unsafe fn readdir(fd: i32, offset: usize, dirp: *mut DirEntry) -> i64 {
    let file_id;

    if fd >= RESERVED_FILE_DESCRIPTORS {
        file_id = (fd - RESERVED_FILE_DESCRIPTORS) as usize;
        if fs::is_dir(file_id).unwrap_or(true) {
            -1
        } else {
            if let Some(mut entry) = fs::read_dir(file_id, offset) {
                entry.id += RESERVED_FILE_DESCRIPTORS as usize;
                *(dirp) = entry;

                0
            } else {
                -1
            }
        }
    } else {
        -1
    }
}

/// Execute a program in a new process.
///
/// # Arguments
/// - `pathname` - Path to the file to execute, must be a valid ELF file.
/// - `argv` - The commandline arguments.
///
/// # Returns
/// The process ID of the new process if the operation was successful, -1 otherwise.
pub unsafe fn exec(pathname: *const u8, argv: *const *const u8) -> i64 {
    let p = scheduler::get_running_process().as_ref().unwrap();
    let args = super::get_args(argv);
    let mut args_str = Vec::new();
    let file_name;
    let file_id;
    let new_pid;

    if let Some(name) = super::get_user_str(p, pathname) {
        file_name = name;
    } else {
        return -1;
    }
    if let Some(id) = fs::get_file_id(file_name, Some(p.cwd())) {
        file_id = id;
    } else {
        return -1;
    };

    for arg in args {
        if let Some(arg) = super::get_user_str(p, *arg) {
            args_str.push(arg);
        } else {
            return -1;
        }
    }
    if let Ok(proc) = scheduler::Process::new_user_process(file_id as u64, p.cwd_path(), &args_str)
    {
        new_pid = proc.pid();
        scheduler::add_to_the_queue(proc);

        new_pid
    } else {
        -1
    }
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
    let layout = Layout::from_size_align(size, allocator::DEFAULT_ALIGNMENT);
    let mut allocation = core::ptr::null_mut();

    if let Ok(layout) = layout {
        allocation = allocator.alloc(layout);
    }

    allocation
}

/// Behaves like `malloc`, but sets the memory to 0.
///
/// # Arguments
/// - `nitems` - The number of elements to be allocated.
/// - `size` - The size of each element.
pub unsafe fn calloc(nitems: usize, size: usize) -> *mut u8 {
    let allocator = scheduler::get_running_process()
        .as_mut()
        .unwrap()
        .allocator();
    let layout = Layout::from_size_align(nitems * size, allocator::DEFAULT_ALIGNMENT);
    let mut allocation = core::ptr::null_mut();

    if let Ok(layout) = layout {
        allocation = allocator.alloc_zeroed(layout);
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
        .dealloc(ptr, Layout::from_size_align(0, 1).unwrap());

    0
}

/// Grow or shrink a block that was allocated with `malloc`.
/// Copies the data from the original block to the new block.
///
/// # Arguments
/// `size` - The new required size of the block.
///
/// # Returns
/// A pointer to a new allocation or null on failure.
pub unsafe fn realloc(ptr: *mut u8, size: usize) -> *mut u8 {
    scheduler::get_running_process()
        .as_mut()
        .unwrap()
        .allocator()
        .realloc(
            ptr,
            Layout::from_size_align_unchecked(size, allocator::DEFAULT_ALIGNMENT),
            size,
        )
}

pub fn sched_yield() -> i64 {
    0
}
