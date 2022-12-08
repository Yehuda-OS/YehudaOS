#![no_std]
#![no_main]

use x86_64::registers::control::Cr3;

mod io;
mod paging;
mod allocator;

/// Kernel Entry Point
///
/// `_start` is defined in the linker script as the entry point for the ELF file.
/// Unless the [`Entry Point`](limine::LimineEntryPointRequest) feature is requested,
/// the bootloader will transfer control to this function.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    let theirs = Cr3::read().0.start_address();
    let table;

    paging::page_allocator::initialize();
    table = paging::virtual_memory_manager::create_page_table();
    paging::map_kernel_address(table);
    paging::create_hhdm(table);

    unsafe { paging::load_tables_to_cr3(table) };

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
