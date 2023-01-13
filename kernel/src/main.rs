#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(strict_provenance)]

extern crate alloc;

mod io;
mod memory;

use crate::memory::allocator::{Allocator, Locked, ALLOCATOR, HEAP_START};

/// Kernel Entry Point
///
/// `_start` is defined in the linker script as the entry point for the ELF file.
/// Unless the [`Entry Point`](limine::LimineEntryPointRequest) feature is requested,
/// the bootloader will transfer control to this function.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    memory::page_allocator::initialize();
    unsafe {
        memory::PAGE_TABLE = memory::virtual_memory_manager::create_page_table();
        memory::map_kernel_address(memory::PAGE_TABLE);
        memory::create_hhdm(memory::PAGE_TABLE);
        memory::load_tables_to_cr3(memory::PAGE_TABLE);
        memory::reclaim_bootloader_memory();
        ALLOCATOR = Locked::<Allocator>::new(Allocator::new(HEAP_START, memory::PAGE_TABLE));
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
