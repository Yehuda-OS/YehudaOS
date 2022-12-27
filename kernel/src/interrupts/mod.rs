pub mod idt;
use idt::IDTEntry;
use lazy_static::lazy_static;

#[repr(C, packed)]
pub struct Idtr {
    limit: u16,
    base: u64,
}

static mut IDT: [IDTEntry; 256] = [IDTEntry::missing(); 256];

pub unsafe fn set_interrupt(index: u8, handler: u64, flags: u8) {
    IDT[index as usize] = IDTEntry::new(handler, flags);
}

pub unsafe fn load_idt() {
    let idtr = Idtr {
        base: (&IDT[0] as *const IDTEntry).addr() as u64,
        limit: (core::mem::size_of::<IDTEntry>() * 256 - 1) as u16,
    };
    core::arch::asm!("cli");
    core::arch::asm!("lidt [{}]", in(reg)&idtr);
    core::arch::asm!("sti");
}
