pub mod idt;
use core::arch::asm;

#[repr(C, packed(2))]
pub struct DescriptorTablePointer {
    pub limit: u16,
    pub base: usize,
}

#[inline]
pub unsafe fn lidt(idt: &DescriptorTablePointer) {
    unsafe {
        asm!("cli");
        asm!("lidt [{}]", in(reg) idt, options(readonly, nostack, preserves_flags));
        asm!("sti")
    }
}
