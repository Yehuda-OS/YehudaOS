pub mod virtual_memory_manager;
pub mod page_allocator;

use x86_64::PhysAddr;

const HHDM_OFFSET: u64 = 0xffff_8000_0000_0000;

pub unsafe fn load_tables_to_cr3(p4_addr: PhysAddr) {
    use x86_64::{registers::control::{Cr3, Cr3Flags}, structures::paging::{Size4KiB, PhysFrame}};
    // The page frame allocator will make sure that the page size will be 4KiB aligned
    Cr3::write(
        PhysFrame::<Size4KiB>::from_start_address(p4_addr).unwrap(), 
        Cr3Flags::empty()
    ); 
}