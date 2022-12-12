#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;
use limine::{LimineFramebufferRequest, LimineTerminalRequest};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{PageTableFlags, PhysFrame, Size1GiB, Size2MiB, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::memory::virtual_memory_manager::virtual_to_physical;

mod io;
mod memory;

/// Kernel Entry Point
///
/// `_start` is defined in the linker script as the entry point for the ELF file.
/// Unless the [`Entry Point`](limine::LimineEntryPointRequest) feature is requested,
/// the bootloader will transfer control to this function.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    let theirs = Cr3::read().0.start_address();

    unsafe {
        memory::allocator::ALLOCATOR =
            memory::allocator::Locked::<memory::allocator::Allocator>::new(
                memory::allocator::Allocator::new(memory::allocator::HEAP_START, theirs),
            )
    };

    memory::page_allocator::initialize();
    unsafe {
        memory::PAGE_TABLE = memory::virtual_memory_manager::create_page_table();
        memory::map_kernel_address(memory::PAGE_TABLE);
        memory::create_hhdm(memory::PAGE_TABLE);
        memory::virtual_memory_manager::map_address(
            memory::PAGE_TABLE,
            VirtAddr::new(0x000000000002910b),
            PhysFrame::<Size1GiB>::containing_address(PhysAddr::new(0x0)),
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::HUGE_PAGE,
        );
        memory::virtual_memory_manager::map_address(
            memory::PAGE_TABLE,
            VirtAddr::new(0x00000000fd000190),
            PhysFrame::<Size2MiB>::containing_address(PhysAddr::new(0x00000000fd000000)),
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::HUGE_PAGE,
        );
        memory::load_tables_to_cr3(memory::PAGE_TABLE);
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
