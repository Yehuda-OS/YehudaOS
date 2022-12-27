use x86_64::registers::segmentation::{Segment, CS};

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct IDTEntry {
    ptr_low: u16,
    selector: u16,
    ist: u8,
    flags: u8,
    ptr_mid: u16,
    ptr_high: u32,
    reserved: u32, // zero
}

impl IDTEntry {
    pub const fn missing() -> Self {
        Self {
            ptr_low: 0,
            selector: 0,
            ist: 0,
            flags: 0,
            ptr_mid: 0,
            ptr_high: 0,
            reserved: 0,
        }
    }

    pub fn new(handler: u64, flags: u8) -> Self {
        Self {
            ptr_low: (handler & 0xFFFF) as u16,
            selector: CS::get_reg().0,
            ist: 0,
            flags: flags,
            ptr_mid: ((handler >> 16) & 0xFFFF) as u16,
            ptr_high: ((handler >> 32) & 0xFFFF_FFFF) as u32,
            reserved: 0,
        }
    }
}
