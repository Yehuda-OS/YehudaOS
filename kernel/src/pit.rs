use super::io;
use crate::scheduler;
use x86_64::structures::idt::InterruptStackFrame;

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

pub unsafe extern "C" fn pit_handler(frame: &InterruptStackFrame) {
    let curr = scheduler::get_running_process().as_mut().unwrap();

    curr.instruction_pointer = frame.instruction_pointer.as_u64();
    curr.stack_pointer = frame.stack_pointer.as_u64();
    curr.flags = frame.cpu_flags;

    scheduler::switch_current_process();
    super::idt::PICS.lock().notify_end_of_interrupt(0x20);
    scheduler::load_from_queue();
}
