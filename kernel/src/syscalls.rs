use super::io;

use core::arch::asm;

const STAR: u32 = 0xc0000081;
const LSTAR: u32 = 0xc0000082;

pub unsafe fn initialize() {
    let rip = handler as u64;
    let cs = u64::from(super::gdt::KERNEL_CODE) << 32;

    io::wrmsr(LSTAR, rip);
    io::wrmsr(STAR, cs);
}

pub unsafe fn handler() {
    crate::println!("A syscall occured");
}
