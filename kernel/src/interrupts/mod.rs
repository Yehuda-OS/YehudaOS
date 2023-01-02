use crate::{print, println};
use bit_field::BitField;
use core::arch::asm;
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use x86_64::addr::VirtAddr;
use x86_64::instructions::segmentation;
use x86_64::structures::gdt::SegmentSelector;
use x86_64::PrivilegeLevel;

////////////////////////////////////////Variables///////////////////////////////////////
enum idt_indexes {
    div_0 = 0,
    breakpoint = 3,
    double_fault = 8,
    timer = 32,
}

pub static pics: spin::Mutex<ChainedPics> = spin::Mutex::new(unsafe { ChainedPics::new(32, 40) });

lazy_static! {
    pub static ref IDT: Idt = {
        let mut idt = Idt::new();

        idt.set_handler(idt_indexes::div_0 as u8, divide_by_zero_wrapper as u64);
        idt.set_handler(idt_indexes::breakpoint as u8, breakpoint_wrapper as u64);
        idt.set_handler(idt_indexes::double_fault as u8, double_fault_wrapper as u64);
        idt.set_handler(idt_indexes::timer as u8, timer_wrapper as u64);
        idt
    };
}
///////////////////////////////////////////////////Structs///////////////////////////////////////

#[derive(Debug)]
#[repr(C)]
pub struct ExceptionStackFrame {
    instruction_pointer: u64,
    code_segment: u64,
    cpu_flags: u64,
    stack_pointer: u64,
    stack_segment: u64,
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

pub type HandlerFunc = extern "C" fn() -> !;

///////////////////////////////////////////////////impl's//////////////////////////////////////////

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
            gdt_selector: gdt_selector,
            pointer_low: pointer as u16,
            pointer_middle: (pointer >> 16) as u16,
            pointer_high: (pointer >> 32) as u32,
            options: EntryOptions::new(),
            reserved: 0,
        }
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

    pub fn set_handler(&mut self, entry: u8, handler: u64) -> EntryOptions {
        self.0[entry as usize] = Entry::new(segmentation::cs(), handler);
        let mut options = self.0[entry as usize].options;
        options
    }

    pub fn load(&'static self) {
        use core::mem::size_of;
        use x86_64::instructions::tables::{lidt, DescriptorTablePointer};

        unsafe {
            let ptr = DescriptorTablePointer {
                base: VirtAddr::new_unsafe(self as *const _ as u64),
                limit: (size_of::<Self>() - 1) as u16,
            };
            lidt(&ptr)
        };
    }
}

///////////////////////////////////////////////////handlers and other functions////////////////////////////////////////

#[naked]
#[no_mangle]
pub extern "C" fn divide_by_zero_wrapper() -> ! {
    unsafe {
        asm!(
            "mov rdi, rsp; call divide_by_zero_handler",
            options(noreturn)
        );
    }
}

#[no_mangle]
pub extern "C" fn divide_by_zero_handler(stack_frame: &ExceptionStackFrame) -> ! {
    println!("\nEXCEPTION: DIVIDE BY ZERO\n{:#?}", unsafe {
        &*stack_frame
    });
    loop {}
}

#[naked]
#[no_mangle]
pub extern "C" fn breakpoint_wrapper() -> ! {
    unsafe {
        asm!(
            "
            push rax;
            push rcx;
            push rdx;
            push rsi;
            push rdi;
            push r8;
            push r9;
            push r10;
            push r11;
    
            mov rdi, rsp; 
            add rdi, 9*8
            call breakpoint_handler;
            
            pop r11;
            pop r10;
            pop r9;
            pop r8;
            pop rdi;
            pop rsi;
            pop rdx;
            pop rcx;
            pop rax;
            iretq",
            options(noreturn)
        );
    }
}

#[no_mangle]
extern "C" fn breakpoint_handler(stack_frame: &ExceptionStackFrame) {
    print!("EXCEPTION: BREAKPOINT");
}

#[naked]
#[no_mangle]
pub extern "C" fn timer_wrapper() -> ! {
    unsafe {
        asm!(
            "
            push rax;
            push rcx;
            push rdx;
            push rsi;
            push rdi;
            push r8;
            push r9;
            push r10;
            push r11;
    
            mov rdi, rsp; 
            add rdi, 9*8
            call timer_handler;
            
            pop r11;
            pop r10;
            pop r9;
            pop r8;
            pop rdi;
            pop rsi;
            pop rdx;
            pop rcx;
            pop rax;
            iretq",
            options(noreturn)
        );
    }
}

#[naked]
#[no_mangle]
pub extern "C" fn keyboard_wrapper() -> ! {
    unsafe {
        asm!(
            "
            push rax;
            push rcx;
            push rdx;
            push rsi;
            push rdi;
            push r8;
            push r9;
            push r10;
            push r11;
    
            mov rdi, rsp; 
            add rdi, 9*8
            call keyboard_handler;
            
            pop r11;
            pop r10;
            pop r9;
            pop r8;
            pop rdi;
            pop rsi;
            pop rdx;
            pop rcx;
            pop rax;
            iretq",
            options(noreturn)
        );
    }
}

#[no_mangle]
extern "C" fn timer_handler() {
    //print!("4");
    unsafe {
        pics.lock()
            .notify_end_of_interrupt(idt_indexes::timer as u8);
    }
}

#[naked]
#[no_mangle]
pub extern "C" fn double_fault_wrapper() -> ! {
    unsafe {
        asm!("mov rdi, rsp; call double_fault_handler", options(noreturn));
    }
}

#[no_mangle]
extern "C" fn double_fault_handler(stack_frame: &ExceptionStackFrame) -> ! {
    print!("EXCEPTION: double fault occured");
    loop {}
}

pub fn init() {
    IDT.load();
}
