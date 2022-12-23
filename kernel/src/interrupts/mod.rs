pub mod idt;
use core::arch::asm;
use x86_64::VirtAddr;

#[inline]
pub unsafe fn lidt(idt: *const idt::Descriptor) {
    unsafe {
        asm!("lidt [{}]", in(reg) idt, options(readonly, nostack, preserves_flags));
    }
}
