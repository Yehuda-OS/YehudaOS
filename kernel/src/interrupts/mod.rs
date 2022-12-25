pub mod idt;

use lazy_static::lazy_static;
use x86_64::instructions::segmentation::{Segment, CS};
use x86_64::instructions::tables::DescriptorTablePointer;
use x86_64::structures::gdt::SegmentSelector;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Descriptor {
    ptr_low: u16,
    selector: SegmentSelector,
    ist: u8, // bits 0..2 holds Interrupt Stack Table offset, rest of bits zero.
    flags: idt::DescriptorFlags, // gate type, dpl, and p fields
    ptr_middle: u16,
    ptr_high: u32,
    zero: u32, // reserved, always zero
}

pub struct Idt(pub [Descriptor; 256]);

impl Descriptor {
    pub fn missing() -> Self {
        Self {
            ptr_low: 0,
            selector: SegmentSelector::new(0, x86_64::PrivilegeLevel::Ring0),
            ist: 0,
            flags: idt::DescriptorFlags::minimal(),
            ptr_middle: 0,
            ptr_high: 0,
            zero: 0,
        }
    }

    pub fn new(selector: x86_64::structures::gdt::SegmentSelector, pointer: u64) -> Self {
        Self {
            ptr_low: pointer as u16,
            selector: selector,
            ist: 0,
            flags: idt::DescriptorFlags::new(),
            ptr_middle: (pointer >> 16) as u16,
            ptr_high: (pointer >> 32) as u32,
            zero: 0,
        }
    }

    unsafe fn set_handler(&mut self, addr: u64, flags: idt::DescriptorFlags) {
        self.ptr_low = (addr & 0xffff) as u16;
        self.selector = CS::get_reg();
        self.ist = 0;
        self.flags = flags;
        self.ptr_middle = ((addr >> 16) & 0xffff) as u16;
        self.ptr_high = ((addr >> 32) & 0xffff_ffff) as u32;
        self.zero = 0;
    }
}

impl Idt {
    pub fn new() -> Self {
        Self([Descriptor::missing(); 256])
    }

    pub fn set_handler(&mut self, index: u8, addr: u64) {
        self.0[index as usize] = Descriptor::new(CS::get_reg(), addr);
    }

    pub fn load(&self) {
        let ptr: DescriptorTablePointer = DescriptorTablePointer {
            limit: (core::mem::size_of::<Self>() - 1) as u16,
            base: x86_64::VirtAddr::new(self as *const _ as u64),
        };

        unsafe { x86_64::instructions::tables::lidt(&ptr) };
    }
}

lazy_static! {
    static ref IDT: Idt = {
        let mut idt = Idt::new();

        idt.set_handler(0, crate::divide_by_zero_handler as u64);

        idt
    };
}

pub fn init() {
    IDT.load();
}
