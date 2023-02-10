use super::io;
use super::scheduler;
use crate::iostream::STDIN;
use core::slice;
use fs_rs::fs::is_dir;
use fs_rs::fs::read as fread;

const EFER: u32 = 0xc0000080;
const STAR: u32 = 0xc0000081;
const LSTAR: u32 = 0xc0000082;
const STDIN_DESCRIPTOR: i32 = 0;
const STDOUT_DESCRIPTOR: i32 = 1;
const STDERR_DESCRIPTOR: i32 = 2;
const TO_SUB_FROM_FD_TO_GET_FILE_ID: i32 = 3;

mod syscall {
    pub const EXIT: u64 = 0x3c;
}

pub unsafe fn initialize() {
    let rip = handler as u64;
    let cs = u64::from(super::gdt::KERNEL_CODE) << 32;

    io::wrmsr(LSTAR, rip);
    io::wrmsr(STAR, cs);
    // Enable syscalls by setting the first bit of the EFER MSR
    io::wrmsr(EFER, 1);
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
pub fn handle_syscall(registers: &scheduler::Registers) -> i64 {
    let syscall_number = registers.rax;
    let arg0 = registers.rdi;
    let arg1 = registers.rsi;
    let arg2 = registers.rdx;
    let arg3 = registers.r10;
    let arg4 = registers.r8;
    let arg5 = registers.r9;

    match syscall_number {
        _ => -1,
    }
}

pub unsafe fn int_0x80_handler() {
    let registers = super::scheduler::save_context();

    handle_syscall(&registers);

    loop {}
}

/// Update the registers of a proccess when a syscall occurs.
/// 
/// # Arguments
/// - `proc` - A mutable reference to the process.
/// - `registers` - The process' registers.
fn update_registers(proc: &mut scheduler::Process, registers: &scheduler::Registers) {
    proc.registers = *registers;
    // The `syscall` instruction saves the instruction pointer in `rcx` and the cpu flags in `r11`.
    proc.instruction_pointer = proc.registers.rcx;
    proc.flags = proc.registers.r11;
    // SAFETY: `rbp` holds the value of the stack pointer after pushing the original `rbp`.
    unsafe {
        core::arch::asm!("
        mov {0}, rbp
        add {0}, 8
        ",
            out(reg)proc.stack_pointer,
        );
    }
}

pub unsafe fn handler() -> ! {
    let mut registers = scheduler::save_context();
    let mut currently_running = scheduler::get_running_process();

    // Disable interrupts while handling a syscall.
    core::arch::asm!("cli");
    crate::println!("A syscall occured");

    // If the syscall is `exit` it must be handled here because we have the reference to the
    // currently running process here.
    if registers.rax == syscall::EXIT {
        *currently_running = None;
    } else {
        registers.rax = handle_syscall(&registers) as u64;
        // UNWRAP: Syscalls should not be called from inside the kernel.
        update_registers(currently_running.as_mut().unwrap(), &registers);
    }

    scheduler::load_from_queue();
}

/// implementation for `read` syscall
///
/// # Arguments
/// - `fd` - the file descriptor
unsafe fn read(fd: i32, buf: *mut u8, count: usize) -> isize {
    if fd < 0 {
        return -1;
    }

    let mut buf = alloc::string::String::new();
    if buf.as_ptr().is_null() {
        return -1;
    }
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
