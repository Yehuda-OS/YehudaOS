pub mod idt;
use core::arch::asm;
use x86_64::VirtAddr;

pub struct DescriptorTablePointer {
    pub limit: u16,
    pub base: usize,
}

#[inline]
pub unsafe fn lidt(idt: &DescriptorTablePointer) {
    unsafe {
        asm!("lidt [{}]", in(reg) idt, options(readonly, nostack, preserves_flags));
        asm!("sti")
    }
}
