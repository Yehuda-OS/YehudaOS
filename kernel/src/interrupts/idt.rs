use x86_64::instructions::segmentation::{Segment, CS};

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
    pub const MISSING: Descriptor = Descriptor {
        ptr_low: 0,
        selector: 0,
        ist: 0,
        flags: 0,
        ptr_middle: 0,
        ptr_high: 0,
        zero: 0,
    };

    pub const fn new(pointer: u64, flags: u8) -> Self {
        Self {
            ptr_low: (pointer & 0xffff) as u16,
            selector: 0x08,
            ist: 0,
            flags: flags,
            ptr_middle: ((pointer >> 16) & 0xffff) as u16,
            ptr_high: ((pointer >> 32) & 0xffff_ffff) as u32,
            zero: 0,
        }
    }

    pub unsafe fn set_handler(&mut self, addr: u64, flags: u8) {
        self.ptr_low = (addr & 0xffff) as u16;
        self.selector = CS::get_reg().0;
        self.ist = 0;
        self.flags = flags;
        self.ptr_middle = ((addr >> 16) & 0xffff) as u16;
        self.ptr_high = ((addr >> 32) & 0xffff_ffff) as u32;
        self.zero = 0;
    }
}
