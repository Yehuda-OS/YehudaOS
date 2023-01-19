
/// Start the system timer.
/// 
/// # Arguments
/// - `tps` - The required ticks per second, must be 18 or greater.
/// 
/// # Safety
/// This operation starts the system timer so it requires a valid handler in the IDT to be loaded.
pub unsafe fn start(tps: u32) {
    let divisor = (TICKS_PER_SECOND / tps) as u16;
    let low = (divisor & 0xff) as u8;
    let high = ((divisor >> 8) & 0xff) as u8;

    io::outb(PIT_COMMAND_PORT, PIT_COMMAND);
    io::outb(0x40, low);
    io::outb(0x40, high);
}

