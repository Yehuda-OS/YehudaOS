pub mod page_allocator;
pub mod virtual_memory_manager;

use limine::LimineMemmapRequest;
use x86_64::PhysAddr;

const HHDM_OFFSET: u64 = 0xffff_8000_0000_0000;
static MEMMAP: LimineMemmapRequest = LimineMemmapRequest::new(0);

pub unsafe fn load_tables_to_cr3(p4_addr: PhysAddr) {
    use x86_64::{
        registers::control::{Cr3, Cr3Flags},
        structures::paging::{PhysFrame, Size4KiB},
    };
    Cr3::write(
        // UNWRAP: The page frame allocator will make sure that
        // the page size will be 4KiB aligned
        PhysFrame::<Size4KiB>::from_start_address(p4_addr).unwrap(),
        Cr3Flags::empty(),
    );
}
