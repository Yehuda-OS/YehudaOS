use x86_64::instructions::segmentation::{Segment, CS};
use x86_64::VirtAddr;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Descriptor {
    ptr_low: u16,
    selector: u16,
    ist: u8,   // bits 0..2 holds Interrupt Stack Table offset, rest of bits zero.
    flags: u8, // gate type, dpl, and p fields
    ptr_middle: u16,
    ptr_high: u32,
    zero: u32, // reserved, always zero
}

impl Descriptor {
    pub const fn new(pointer: u64, flags: u8, ist: u8) -> Self {
        Self {
            ptr_low: (pointer & 0xffff) as u16,
            selector: 0x08,
            ist: ist,
            flags: flags,
            ptr_middle: ((pointer & 0xffff_0000) >> 16) as u16,
            ptr_high: ((pointer & 0xffff_ffff_0000_0000) >> 32) as u32,
            zero: 0,
        }
    }

    pub const fn empty() -> Self {
        Self {
            ptr_low: 0,
            selector: 0,
            ist: 0,
            flags: 0,
            ptr_middle: 0,
            ptr_high: 0,
            zero: 0,
        }
    }

    pub unsafe fn set_handler(&mut self, addr: VirtAddr, flags: u8) {
        let addr = addr.as_u64();

        self.ptr_low = addr as u16;
        self.ptr_middle = (addr >> 16) as u16;
        self.ptr_high = (addr >> 32) as u32;

        self.selector = CS::get_reg().0;

        self.flags = flags;
    }
}
