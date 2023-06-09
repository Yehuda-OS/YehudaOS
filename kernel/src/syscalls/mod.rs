use alloc::string::String;
use alloc::vec::Vec;
use x86_64::VirtAddr;

use super::io;
use super::scheduler;
use crate::memory;
use core::arch::asm;
use core::u8;
use fs_rs::fs::DirEntry;

mod handlers;

const EFER: u32 = 0xc0000080;
const STAR: u32 = 0xc0000081;
const LSTAR: u32 = 0xc0000082;
const FMASK: u32 = 0xc0000084;
pub const KERNEL_GS_BASE: u32 = 0xc0000102;

static mut KERNEL_STACK: u64 = 0;

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
    _arg4: u64,
    _arg5: u64,
) -> i64 {
    match syscall_number {
        handlers::READ => {
            handlers::read(arg0 as i32, arg1 as *mut u8, arg2 as usize, arg3 as usize)
        }
        handlers::WRITE => {
            handlers::write(arg0 as i32, arg1 as *const u8, arg2 as usize, arg3 as usize)
        }
        handlers::EXEC => handlers::exec(arg0 as *const u8, arg1 as *const *const u8),
        handlers::MALLOC => handlers::malloc(arg0 as usize) as i64,
        handlers::CALLOC => handlers::calloc(arg0 as usize, arg1 as usize) as i64,
        handlers::FREE => handlers::free(arg0 as *mut u8),
        handlers::REALLOC => handlers::realloc(arg0 as *mut u8, arg1 as usize) as i64,
        handlers::SCHED_YIELD => handlers::sched_yield(),
        handlers::EXIT => handlers::exit(arg0 as i32),
        handlers::GET_CURRENT_DIR_NAME => handlers::get_current_dir_name() as i64,
        handlers::CHDIR => handlers::chdir(arg0 as *const u8),
        handlers::CREAT => handlers::creat(arg0 as *mut u8, arg1 != 0) as i64,
        handlers::OPEN => handlers::open(arg0 as *const u8) as i64,
        handlers::FSTAT => handlers::fstat(arg0 as i32, arg1 as *mut handlers::Stat),
        handlers::WAITPID => handlers::waitpid(arg0 as i64, arg1 as *mut i32),
        handlers::REMOVE_FILE => handlers::remove_file(arg0 as *mut u8),
        handlers::TRUNCATE => handlers::truncate(arg0 as *const u8, arg1),
        handlers::FTRUNCATE => handlers::ftruncate(arg0 as i32, arg1),
        handlers::READ_DIR => handlers::readdir(arg0 as i32, arg1 as usize, arg2 as *mut DirEntry),
        _ => -1,
    }
}

/// Returns the length of a null-terminated string.
///
/// # Arguments
/// - `buffer` - Pointer to the string's data.
///
/// # Safety
/// Might produce a page fault if the string isn't null-terminated or if the buffer points to
/// unmapped memory.
unsafe fn strlen(buffer: *const u8) -> usize {
    let mut i = 0;

    while *buffer.add(i) != 0 {
        i += 1;
    }

    i
}

/// Get the arguments array from a raw pointer.
///
/// # Arguments
/// - `argv` - The pointer to the arguments array.
///
/// # Safety
/// Assumes that `argv` points to a null-terminated array.
unsafe fn get_args(argv: *const *const u8) -> &'static [*const u8] {
    let mut len = 0;

    while !(*argv.add(len)).is_null() {
        len += 1;
    }

    core::slice::from_raw_parts(argv, len)
}

/// Get the absolute path to a file from a relative path.
///
/// # Arguments
/// - `path` - A path to a file.
///
/// # Returns
/// The absolute path to the file that `path` refers to.
fn get_absolute_path(path: &str) -> String {
    let components = path.split('/');
    let mut stack = Vec::new();
    let mut result = String::new();

    for component in components {
        match component {
            "." => continue,
            ".." => {
                if stack.len() > 1 {
                    stack.pop();
                }
            }
            _ => {
                stack.push(component);
            }
        }
    }
    result.push_str(&stack.join("/"));
    if result.is_empty() {
        result.push('/');
    }

    result
}

/// Get a slice borrow from a user buffer.
///
/// # Arguments
/// - `process` - The user process that sent the buffer.
/// - `buffer` - Pointer to the data.
/// - `len` - Length of the data.
///
/// # Returns
/// The user's buffer on success or `None` if the buffer is outside the user's memory or isn't
/// mapped to a physical address.
///
/// # Safety
/// Assumes the buffer is valid and actually of length `len`.
unsafe fn get_user_buffer(
    process: &scheduler::Process,
    buffer: *const u8,
    len: usize,
) -> Option<&[u8]> {
    let page;

    if buffer.is_null() || buffer as u64 >= memory::HHDM_OFFSET {
        None
    } else {
        page = memory::vmm::virtual_to_physical(process.page_table, VirtAddr::new(buffer as u64))
            .ok()?;

        Some(core::slice::from_raw_parts(
            (page.as_u64() + memory::HHDM_OFFSET) as *const u8,
            len,
        ))
    }
}

/// Mutable version of `get_user_buffer`.
unsafe fn get_user_buffer_mut(
    process: &scheduler::Process,
    buffer: *mut u8,
    len: usize,
) -> Option<&mut [u8]> {
    let buf = get_user_buffer(process, buffer, len)?;

    Some(core::slice::from_raw_parts_mut(
        buf.as_ptr() as *mut u8,
        buf.len(),
    ))
}

/// Returns a user string from a pointer or `None` if the data is invalid.
///
/// # Arguments
/// `process` - The process that owns the data.
/// `buffer` - The buffer the process has sent.
unsafe fn get_user_str(process: &scheduler::Process, buffer: *const u8) -> Option<&str> {
    core::str::from_utf8(get_user_buffer(process, buffer, strlen(buffer))?).ok()
}

pub unsafe fn int_0x80_handler() {
    let proc = scheduler::get_running_process().as_mut().unwrap();

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

    proc.registers.rax = handle_syscall(
        proc.registers.rax,
        proc.registers.rdi,
        proc.registers.rsi,
        proc.registers.rdx,
        proc.registers.r10,
        proc.registers.r8,
        proc.registers.r9,
    ) as u64;

    scheduler::switch_current_process();
    scheduler::load_from_queue();
}
