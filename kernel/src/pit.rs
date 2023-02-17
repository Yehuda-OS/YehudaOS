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
    core::arch::asm!("sti");
}

pub extern "x86-interrupt" fn handler(_stack_frame: x86_64::structures::idt::InterruptStackFrame) {
    unsafe {
        super::idt::PICS.lock().notify_end_of_interrupt(0x20);
    }
}
