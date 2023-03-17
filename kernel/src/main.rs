#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(strict_provenance)]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]
#![feature(naked_functions)]
#![feature(asm_sym)]

extern crate alloc;

use fs_rs::fs;

mod gdt;
mod idt;
mod io;
mod iostream;
mod memory;
mod mutex;
mod pit;
mod queue;
mod scheduler;
mod syscalls;
mod terminal;

/// Kernel Entry Point
///
/// `_start` is defined in the linker script as the entry point for the ELF file.
/// Unless the [`Entry Point`](limine::LimineEntryPointRequest) feature is requested,
/// the bootloader will transfer control to this function.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    memory::page_allocator::initialize();
    unsafe {
        // UNWRAP: There's no point in continuing without a valid page table.
        memory::PAGE_TABLE = memory::vmm::create_page_table()
            .expect("Not enough free memory for a kernel's page table");
        memory::map_kernel_address().unwrap();
        memory::create_hhdm(memory::PAGE_TABLE).unwrap();
        memory::map_bootloader_memory().unwrap();
        memory::load_tables_to_cr3(memory::PAGE_TABLE);
        memory::allocator::ALLOCATOR
            .lock()
            .set_page_table(memory::PAGE_TABLE);
        gdt::create();
        gdt::activate();
        fs::init();
        scheduler::load_tss();
        idt::IDT.load();
        syscalls::initialize();
        pit::start(19);
    }
    println!("Hello world");

    hcf();
}

#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    hcf();
}

/// Die, spectacularly.
pub fn hcf() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
