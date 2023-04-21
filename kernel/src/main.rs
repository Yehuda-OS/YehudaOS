#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(strict_provenance)]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]
#![feature(naked_functions)]
#![feature(asm_sym)]
#![feature(const_btree_new)]

extern crate alloc;

use alloc::vec::Vec;
use fs_rs::fs::{self, FsError};
use limine::LimineFramebufferRequest;

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

const LOGO_SIZE: u64 = 500;

static FRAMEBUFFER: LimineFramebufferRequest = LimineFramebufferRequest::new(0);

pub unsafe fn print_logo() -> Option<()> {
    let framebuffer = &FRAMEBUFFER.get_response().get()?.framebuffers()[0];
    let address = framebuffer.address.as_ptr()?;
    let logo = include_bytes!("../../YehudaOS.rgba");
    let row_offset = framebuffer.width - LOGO_SIZE;

    for y in 0..LOGO_SIZE {
        for x in 0..LOGO_SIZE {
            let offset = (y * LOGO_SIZE + x) as usize * 4;
            let screen_offset = (y * framebuffer.width + x + row_offset) as usize * 4;
            *address.add(screen_offset) = logo[offset];
            *address.add(screen_offset + 1) = logo[offset + 1];
            *address.add(screen_offset + 2) = logo[offset + 2];
            *address.add(screen_offset + 3) = logo[offset + 3];
        }
    }

    Some(())
}

pub unsafe fn initialize_everything() {
    memory::page_allocator::initialize();
    // UNWRAP: There's no point in continuing without a valid page table.
    memory::PAGE_TABLE =
        memory::vmm::create_page_table().expect("Not enough free memory for a kernel's page table");
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

/// Add a file to the file system.
///
/// # Arguments
/// - `name` - The name/path of the file.
/// - `content` - The content of the file.
///
/// # Returns
/// The inode ID of the new file on success or `FsError` on error.
pub unsafe fn add_executable(name: &str, content: &[u8]) -> Result<usize, FsError> {
    let file_id = fs::create_file(name, false, None)?;

    fs::write(file_id, content, 0)?;

    Ok(file_id)
}

pub unsafe fn add_processes() -> Result<(), FsError> {
    let shell = add_executable("/shell", include_bytes!("../bin/shell"))?;

    add_executable("/touch", include_bytes!("../bin/touch"))?;
    add_executable("/mkdir", include_bytes!("../bin/mkdir"))?;
    add_executable("/ls", include_bytes!("../bin/ls"))?;
    add_executable("/rm", include_bytes!("../bin/rm"))?;
    add_executable("/repeat", include_bytes!("../bin/repeat"))?;
    add_executable("/multiprocessing", include_bytes!("../bin/multiprocessing"))?;
    add_executable("/rmdir", include_bytes!("../bin/rmdir"))?;

    scheduler::add_to_the_queue(
        scheduler::Process::new_user_process(shell as u64, "/", &Vec::new())
            .map_err(|_| FsError::NotEnoughDiskSpace)?,
    );
    scheduler::add_to_the_queue(
        scheduler::Process::new_kernel_task(
            scheduler::terminator::terminate_from_queue,
            core::ptr::null_mut(),
        )
        .expect("Error: failed to load processes terminator"),
    );

    Ok(())
}

/// Kernel Entry Point
///
/// `_start` is defined in the linker script as the entry point for the ELF file.
/// Unless the [`Entry Point`](limine::LimineEntryPointRequest) feature is requested,
/// the bootloader will transfer control to this function.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        initialize_everything();
        print_logo();
        add_processes().expect("failed to add executables");
        println!("Welcome to YehudaOS!");
        scheduler::load_from_queue();
    }
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
