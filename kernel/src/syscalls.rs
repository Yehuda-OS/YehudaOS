use super::io;
use super::scheduler;
use crate::iostream::STDIN;
use crate::memory;
use core::arch::asm;
use core::slice;
use fs_rs::fs::is_dir;
use fs_rs::fs::read as fread;

const EFER: u32 = 0xc0000080;
const STAR: u32 = 0xc0000081;
const LSTAR: u32 = 0xc0000082;
const FMASK: u32 = 0xc0000084;
pub const KERNEL_GS_BASE: u32 = 0xc0000102;

const STDIN_DESCRIPTOR: i32 = 0;
const STDOUT_DESCRIPTOR: i32 = 1;
const STDERR_DESCRIPTOR: i32 = 2;
const TO_SUB_FROM_FD_TO_GET_FILE_ID: i32 = 3;

static mut KERNEL_STACK: u64 = 0;

mod syscall {
    pub const EXIT: u64 = 0x3c;
    pub const READ: u64 = 0;
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
    *scheduler::get_running_process() = None;

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
        syscall::EXIT => exit(arg0 as i32),
        _ => -1,
    }
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

/// implementation for `read` syscall
///
/// # Arguments
/// - `fd` - the file descriptor
/// - `_buf` - the buffer to write into
/// - `count` - the count of bytes to rea
///
/// # Returns
/// 0 if the operation was successful, -1 otherwise
unsafe fn read(fd: i32, _buf: *mut u8, count: usize) -> isize {
    if fd < 0 {
        return -1;
    }

    if _buf.is_null() {
        return -1;
    }
    let mut buf = alloc::string::String::from_raw_parts(_buf, count, 1024); // capacity of 1 KB
    if fd < 3 && fd >= 0 {
        match fd {
            STDIN_DESCRIPTOR => return STDIN.read_line(&mut buf) as isize,
            STDOUT_DESCRIPTOR => return 0, // STDOUT still not implemented
            STDERR_DESCRIPTOR => return 0, // STDERR still not implemented
            _ => {}
        }
    }

    let buffer = unsafe { slice::from_raw_parts_mut(buf.as_mut_ptr(), count) };
    match fread((fd - TO_SUB_FROM_FD_TO_GET_FILE_ID) as usize, buffer, 0) {
        Some(b) => {
            if is_dir((fd - TO_SUB_FROM_FD_TO_GET_FILE_ID) as usize) {
                return -1;
            }
            b as isize
        }
        None => -1,
    }
}
