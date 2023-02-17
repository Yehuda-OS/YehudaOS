use core::arch::asm;

use x86_64::structures::idt::InterruptStackFrame;

use crate::{memory, scheduler};

use super::io;

const TICKS_PER_SECOND: u32 = 1193182;
const PIT_COMMAND_PORT: u16 = 0x43;
const PIT_COMMAND: u8 = 0x36;
const PIT_CHANNEL0: u16 = 0x40;

/// Start the system timer and enables interrupts.
///
/// # Arguments
/// - `tps` - The required ticks per second, must be 18 or greater.
///
/// # Safety
/// This operation starts the system timer so it requires a valid handler in the IDT to be loaded.
pub unsafe fn start(tps: u32) {
    let divisor = (TICKS_PER_SECOND / tps) as u16;
    let low = (divisor & 0xff) as u8;
    let high = (divisor >> 8) as u8;

    io::outb(PIT_COMMAND_PORT, PIT_COMMAND);
    io::outb(PIT_CHANNEL0, low);
    io::outb(PIT_CHANNEL0, high);
}

/// Save the general purpose registers of the process and run the handler.
///
/// # Safety
/// This operation assumes it was triggered from the pit interrupt
/// during the execution of a process in ring 3.
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
        swapgs
        mov rdi, rsp
        call pit_handler
    ",
        options(noreturn)
    );
}

#[no_mangle]
unsafe extern "C" fn pit_handler(frame: &InterruptStackFrame) {
    let curr = scheduler::get_running_process().as_mut().unwrap();

    memory::load_tables_to_cr3(memory::get_page_table());
    curr.instruction_pointer = (*frame).instruction_pointer.as_u64();
    curr.stack_pointer = (*frame).stack_pointer.as_u64();
    curr.flags = (*frame).cpu_flags;

    crate::print!(".");

    scheduler::switch_current_process();
    super::idt::PICS.lock().notify_end_of_interrupt(0x20);
    scheduler::load_from_queue();
}
