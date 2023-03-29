use super::io;
use super::scheduler;
use crate::iostream::STDIN;
use crate::memory;
use core::alloc::Layout;
use core::arch::asm;
use core::ptr::null_mut;
use core::slice;
use core::u8;
use fs_rs::fs;
use fs_rs::fs::read as fread;
use fs_rs::fs::{get_file_id, is_dir, FILE_NAME_LEN};
use x86_64::VirtAddr;

const EFER: u32 = 0xc0000080;
const STAR: u32 = 0xc0000081;
const LSTAR: u32 = 0xc0000082;
const FMASK: u32 = 0xc0000084;
const ALIGNMENT: usize = 16;
pub const KERNEL_GS_BASE: u32 = 0xc0000102;

const STDIN_DESCRIPTOR: i32 = 0;
const STDOUT_DESCRIPTOR: i32 = 1;
const STDERR_DESCRIPTOR: i32 = 2;
const RESERVED_FILE_DESCRIPTORS: i32 = 3;

static mut KERNEL_STACK: u64 = 0;

mod syscall {
    pub const EXEC: u64 = 0x3b;
    pub const EXIT: u64 = 0x3c;
    pub const READ: u64 = 0;
    pub const MALLOC: u64 = 0x9;
    pub const FREE: u64 = 0xb;
    pub const CREATE_FILE: u64 = 0x2;
    pub const REMOVE_FILE: u64 = 0x57;
    // TODO read, write, ftruncate, read_dir
}

pub unsafe fn initialize() {
    let rip = handler_save_context as u64;
    let cs = u64::from(super::gdt::KERNEL_CODE) << 32;

    KERNEL_STACK = scheduler::get_kernel_stack();

    io::wrmsr(LSTAR, rip);
    io::wrmsr(STAR, cs);
    // Enable syscalls by setting the first bit of the EFER MSR
    io::wrmsr(EFER, 1);
    // Write !0 to the `FMASK` MSR to clear all the bits of `rflags` when a syscall occurs.
    io::wrmsr(FMASK, !0);
    // Write the kernel's stack to the gs register.
    io::wrmsr(KERNEL_GS_BASE, &KERNEL_STACK as *const _ as u64);
    asm!("swapgs");
}

unsafe fn exit(_status: i32) -> i64 {
    crate::scheduler::terminator::add_to_queue(core::ptr::read(
        scheduler::get_running_process().as_mut().unwrap(),
    ));
    core::ptr::write(scheduler::get_running_process(), None);

    return 0;
}

/// Handle the syscall (Perform the action that the process has requested).
///
/// # Arguments
/// - `syscall_number` - The identifier of the syscall, the value stored in `rax`.
/// - `arg0` - Stored in `rdi`.
/// - `arg1` - Stored in `rsi`.
/// - `arg2` - Stored in `rdx`.
/// - `arg3` - Stored in `r10`.
/// - `arg4` - Stored in `r8`.
/// - `arg5` - Stored in `r9`.
unsafe fn handle_syscall(
    syscall_number: u64,
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
) -> i64 {
    match syscall_number {
        syscall::READ => read(arg0 as i32, arg2 as *mut u8, arg2 as usize) as i64,
        syscall::EXEC => exec(arg0 as *const i8),
        syscall::MALLOC => malloc(arg0 as usize) as i64,
        syscall::FREE => free(arg0 as *mut u8),
        syscall::EXIT => exit(arg0 as i32),
        syscall::CREATE_FILE => create_file(arg0 as *mut u8, arg2 > 0) as i64,
        _ => -1,
    }
}

unsafe fn strlen(buffer: *mut u8) -> usize {
    let i = 0;

    while *buffer.add(i) != 0 {
        i += 1;
    }

    i
}

/// Returns a user string from a pointer or `None` if the data is invalid.
///
/// # Arguments
/// `process` - The process that owns the data.
/// `buffer` - The buffer the process has sent.
unsafe fn get_user_str(process: &scheduler::Process, buffer: *mut u8) -> Option<&str> {
    let page;

    if buffer.is_null() || buffer as u64 >= memory::HHDM_OFFSET {
        return None;
    }
    page =
        memory::vmm::virtual_to_physical(process.page_table, VirtAddr::new(buffer as u64)).ok()?;

    core::str::from_utf8(core::slice::from_raw_parts(buffer, strlen(buffer))).ok()
}

pub unsafe fn int_0x80_handler() {
    let mut registers = scheduler::Registers::default();

    registers.rax = handle_syscall(
        registers.rax,
        registers.rdi,
        registers.rsi,
        registers.rdx,
        registers.r10,
        registers.r8,
        registers.r9,
    ) as u64;

    loop {}
}

/// Saves all the registers of the process, restores `rsp` and then calls the handler.
/// Does not load the kernel's page table.
#[naked]
pub unsafe extern "C" fn handler_save_context() {
    asm!(
        "
        mov gs:0, rax
        mov gs:8, rbx
        mov gs:16, rcx
        mov gs:24, rdx
        mov gs:32, rsi
        mov gs:40, rdi
        mov gs:48, rbp
        mov gs:56, r8
        mov gs:64, r9
        mov gs:72, r10
        mov gs:80, r11
        mov gs:88, r12
        mov gs:96, r13
        mov gs:104, r14
        mov gs:112, r15
        mov gs:120, rsp
        swapgs
        mov rsp, gs:0
        swapgs
        call handler
    ",
        options(noreturn)
    );
}

#[no_mangle]
pub unsafe fn handler() -> ! {
    // UNWRAP: Syscalls should not be called from inside the kernel.
    let proc = scheduler::get_running_process().as_mut().unwrap();

    // The `syscall` instruction saves the instruction pointer in `rcx` and the cpu flags in `r11`.
    proc.instruction_pointer = proc.registers.rcx;
    proc.flags = proc.registers.r11;
    memory::load_tables_to_cr3(memory::get_page_table());
    crate::println!("A syscall occured");

    proc.registers.rax = handle_syscall(
        proc.registers.rax,
        proc.registers.rdi,
        proc.registers.rsi,
        proc.registers.rdx,
        proc.registers.r10,
        proc.registers.r8,
        proc.registers.r9,
    ) as u64;

    scheduler::load_from_queue();
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
unsafe fn create_file(path: *mut u8, directory: bool) -> i32 {
    let p = scheduler::get_running_process().as_ref().unwrap();
    let name_str;

    if let Some(name) = get_user_str(p, path) {
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

/// Remove a file from the file system, or remove a directory that must be empty.
///
/// # Arguments
/// - `path` - Path to the file.
/// - `path_len` - Length of the path.
///
/// # Returns
/// 0 if the operation was successful, -1 otherwise.
unsafe fn remove_file(path: *mut u8) -> i64 {
    let p = scheduler::get_running_process().as_ref().unwrap();
    let name_str;

    if let Some(name) = get_user_str(p, path) {
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

/// implementation for `read` syscall
///
/// # Arguments
/// - `fd` - the file descriptor
/// - `user_buffer` - the buffer to write into
/// - `count` - the count of bytes to rea
///
/// # Returns
/// 0 if the operation was successful, -1 otherwise.
unsafe fn read(fd: i32, user_buffer: *mut u8, count: usize) -> i64 {
    let p = scheduler::get_running_process().as_ref().unwrap();
    let buffer;
    let mut buf;

    if fd < 0 || user_buffer.is_null() || user_buffer as u64 >= memory::HHDM_OFFSET {
        return -1;
    }
    if let Ok(page) =
        memory::vmm::virtual_to_physical(p.page_table, VirtAddr::new(user_buffer as u64))
    {
        buf = alloc::string::String::from_raw_parts(
            (page + memory::HHDM_OFFSET).as_u64() as *mut u8,
            count,
            count,
        );
    } else {
        return -1;
    }

    if fd < 3 && fd >= 0 {
        match fd {
            STDIN_DESCRIPTOR => return STDIN.read_line(&mut buf) as i64,
            STDOUT_DESCRIPTOR => return 0, // STDOUT still not implemented
            STDERR_DESCRIPTOR => return 0, // STDERR still not implemented
            _ => {}
        }
    }

    buffer = slice::from_raw_parts_mut(buf.as_mut_ptr(), count);
    match fread((fd - RESERVED_FILE_DESCRIPTORS) as usize, buffer, 0) {
        Some(b) => {
            if is_dir((fd - RESERVED_FILE_DESCRIPTORS) as usize) {
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
unsafe fn exec(name: *const i8) -> i64 {
    let mut len: usize = 0;
    while *(name.add(len)) != 0 {
        len += 1;
        if len > FILE_NAME_LEN {
            return -1;
        }
    }

    let bytes: &[u8] = slice::from_raw_parts(name as *mut u8, len);
    let file_name = if let Ok(v) = core::str::from_utf8(bytes) {
        v
    } else {
        return -1;
    };

    let id = if let Some(id) = get_file_id(file_name, None) {
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
unsafe fn malloc(size: usize) -> *mut u8 {
    let allocator = scheduler::get_running_process()
        .as_mut()
        .unwrap()
        .allocator();
    let layout = Layout::from_size_align(size, ALIGNMENT);
    let mut allocation = null_mut();

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
unsafe fn free(ptr: *mut u8) -> i64 {
    scheduler::get_running_process()
        .as_mut()
        .unwrap()
        .allocator()
        .global_dealloc(ptr, Layout::from_size_align(0, 1).unwrap());

    0
}
