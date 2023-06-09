pub mod keyboard;
mod macros;

use crate::pit::pit_handler;
use crate::syscalls::int_0x80_handler as syscall_handler;
use crate::{interrupt_handler, print, println, scheduler};
use bit_field::BitField;
use core::arch::asm;
use keyboard::handler as keyboard_handler;
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use x86_64::addr::VirtAddr;
use x86_64::structures::gdt::SegmentSelector;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::structures::idt::PageFaultErrorCode;
use x86_64::structures::paging::{PageTableFlags, PhysFrame};
use x86_64::PrivilegeLevel;

const DIV_0: u8 = 0;
const BREAKPOINT: u8 = 3;
const DOUBLE_FAULT: u8 = 8;
const PAGE_FAULT: u8 = 0xE;
const PIC_OFFSET1: u8 = 0x20;
const PIC_OFFSET2: u8 = PIC_OFFSET1 + 8;
const PIT_HANDLER: u8 = 0x20;
const SYSCALL_HANDLER: u8 = 0x80;
const KEYBOARD_HANDLER: u8 = 0x21;

pub static PICS: crate::mutex::Mutex<ChainedPics> =
    crate::mutex::Mutex::new(unsafe { ChainedPics::new(PIC_OFFSET1, PIC_OFFSET2) });

lazy_static! {
    pub static ref IDT: Idt = {
        let mut idt = Idt::new();

        idt.set_handler(
            DIV_0,
            interrupt_handler!(divide_by_zero_handler => div_0) as u64,
        );
        idt.set_handler(
            BREAKPOINT,
            interrupt_handler!(breakpoint_handler => breakpoint) as u64,
        );
        idt.set_handler(
            DOUBLE_FAULT,
            interrupt_handler!(double_fault_handler => d_fault) as u64,
        );
        idt.set_handler(
            PAGE_FAULT,
            interrupt_handler!(page_fault_handler => p_fault) as u64,
        );
        idt.set_handler_entry(
            PIT_HANDLER,
            *Entry::new(
                SegmentSelector::new(crate::gdt::KERNEL_CODE / 8, PrivilegeLevel::Ring0),
                interrupt_handler!(pit_handler => pit_save_context) as u64,
            )
            .set_stack_index(1),
        );
        idt.set_handler_entry(
            KEYBOARD_HANDLER,
            *Entry::new(
                SegmentSelector::new(crate::gdt::KERNEL_CODE / 8, PrivilegeLevel::Ring0),
                interrupt_handler!(keyboard_handler => keyboard) as u64,
            )
            .set_stack_index(1),
        );
        idt.set_handler_entry(
            SYSCALL_HANDLER,
            *Entry::new(
                SegmentSelector::new(crate::gdt::KERNEL_CODE / 8, PrivilegeLevel::Ring0),
                interrupt_handler!(syscall_handler => syscall) as u64,
            )
            .set_stack_index(1),
        );

        idt
    };
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Entry {
    pointer_low: u16,
    gdt_selector: SegmentSelector,
    options: EntryOptions,
    pointer_middle: u16,
    pointer_high: u32,
    reserved: u32,
}

pub struct Idt([Entry; 256]);

#[derive(Debug, Clone, Copy)]
pub struct EntryOptions(u16);

impl EntryOptions {
    fn minimal() -> Self {
        let mut options = 0;
        options.set_bits(9..12, 0b111); // 'must-be-one' bits
        EntryOptions(options)
    }

    fn new() -> Self {
        let mut options = Self::minimal();
        options.set_present(true).disable_interrupts(true);
        options
    }

    pub fn set_present(&mut self, present: bool) -> &mut Self {
        self.0.set_bit(15, present);
        self
    }

    pub fn disable_interrupts(&mut self, disable: bool) -> &mut Self {
        self.0.set_bit(8, !disable);
        self
    }

    pub fn set_privilege_level(&mut self, dpl: u16) -> &mut Self {
        self.0.set_bits(13..15, dpl);
        self
    }

    pub fn set_stack_index(&mut self, index: u16) -> &mut Self {
        self.0.set_bits(0..3, index);
        self
    }
}

impl Entry {
    fn new(gdt_selector: SegmentSelector, handler: u64) -> Self {
        let pointer = handler as u64;
        Entry {
            gdt_selector,
            pointer_low: pointer as u16,
            pointer_middle: (pointer >> 16) as u16,
            pointer_high: (pointer >> 32) as u32,
            options: EntryOptions::new(),
            reserved: 0,
        }
    }

    fn set_stack_index(&mut self, index: u16) -> &mut Self {
        let mut copy = self.options;

        self.options = *copy.set_stack_index(index);

        self
    }

    fn missing() -> Self {
        Entry {
            gdt_selector: SegmentSelector::new(0, PrivilegeLevel::Ring0),
            pointer_low: 0,
            pointer_middle: 0,
            pointer_high: 0,
            options: EntryOptions::minimal(),
            reserved: 0,
        }
    }
}

impl Idt {
    pub fn new() -> Idt {
        Idt([Entry::missing(); 256])
    }

    pub fn set_handler(&mut self, entry: u8, handler: u64) {
        self.0[entry as usize] = Entry::new(
            SegmentSelector::new(crate::gdt::KERNEL_CODE / 8, PrivilegeLevel::Ring0),
            handler,
        );
    }

    pub fn set_handler_entry(&mut self, index: u8, handler: Entry) {
        self.0[index as usize] = handler;
    }

    pub fn load(&'static self) {
        use core::mem::size_of;

        unsafe {
            let ptr = x86_64::structures::DescriptorTablePointer {
                base: VirtAddr::new_unsafe(self as *const _ as u64),
                limit: (size_of::<Self>() - 1) as u16,
            };
            let mut pics = PICS.lock();

            pics.initialize();
            pics.write_masks(0, 0);
            x86_64::instructions::tables::lidt(&ptr)
        };
    }
}

unsafe fn divide_by_zero_handler(stack_frame: &InterruptStackFrame) -> ! {
    crate::memory::load_tables_to_cr3(crate::memory::get_page_table());
    print!("\nEXCEPTION: DIVIDE BY ZERO\n{:#?}", unsafe {
        &*stack_frame
    });
    loop {}
}

unsafe fn breakpoint_handler(stack_frame: &InterruptStackFrame) {
    crate::memory::load_tables_to_cr3(crate::memory::get_page_table());
    print!("EXCEPTION: BREAKPOINT");
    loop {}
}

unsafe fn double_fault_handler(stack_frame: &InterruptStackFrame) -> ! {
    crate::memory::load_tables_to_cr3(crate::memory::get_page_table());
    print!("EXCEPTION: double fault occured");
    loop {}
}

unsafe fn page_fault_handler(
    stack_frame: &InterruptStackFrame,
    error_code: PageFaultErrorCode,
) -> ! {
    let curr = crate::scheduler::get_running_process().as_mut().unwrap();
    let pfault_address = x86_64::registers::control::Cr2::read();

    if pfault_address <= curr.stack_start()
        && pfault_address >= (curr.stack_start() - scheduler::MAX_STACK_SIZE)
    {
        let new_stack_page: PhysFrame;
        match crate::memory::page_allocator::allocate() {
            Some(v) => new_stack_page = v,
            None => {
                *scheduler::get_running_process() = None;
                crate::scheduler::load_from_queue();
            }
        }

        if let Err(_) = crate::memory::vmm::map_address(
            curr.page_table,
            x86_64::registers::control::Cr2::read(),
            new_stack_page,
            PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE,
        ) {
            scheduler::terminator::add_to_queue(
                core::mem::replace(scheduler::get_running_process(), None).unwrap(),
            );
        }

        crate::scheduler::load_from_queue();
    } else {
        crate::memory::load_tables_to_cr3(crate::memory::get_page_table());
        println!("============");
        println!("|Page Fault|");
        println!("============");
        println!(
            "Page fault at address {:#x}",
            x86_64::registers::control::Cr2::read().as_u64()
        );
        println!("Stack Frame: {:#x?}", stack_frame);
        println!("Error Code: {:#x?}", error_code); // the only panic so it will stop after it
        loop {}
    }
}
