#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(strict_provenance)]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]
#![feature(naked_functions)]

extern crate alloc;

mod interrupts;
mod io;
mod memory;

fn test_idt() {
    unsafe {
        core::arch::asm!(
            "
            mov dx, 0;
            div dx
            "
        )
    };
}

extern "C" fn page_fault_handler() -> ! {
    println!("tried to devide by zero");
    loop {}
}

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
        interrupts::IDT.load();
        test_idt();
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
