pub mod page_allocator;
pub mod virtual_memory_manager;

use limine::{
    LimineHhdmRequest, LimineKernelAddressRequest, LimineMemmapEntry, LimineMemmapRequest,
    LimineMemmapResponse, LimineMemoryMapEntryType,
};
use x86_64::{
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{PageSize, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

const PAGE_TABLE_ENTRIES: isize = 512;

static KERNEL_ADDRESS: LimineKernelAddressRequest = LimineKernelAddressRequest::new(0);
static HHDM: LimineHhdmRequest = LimineHhdmRequest::new(0);
static mut HHDM_OFFSET: u64 = 0;
static MEMMAP: LimineMemmapRequest = LimineMemmapRequest::new(0);

/// Unwrap the memory map response from the request.
fn get_memmap() -> &'static LimineMemmapResponse {
    MEMMAP.get_response().get().unwrap()
}

/// Get an entry from the memmory map.
///
/// # Arguments
/// `memmap` - The memory map.
/// `i` - The offset of the entry in the memory map.
///
/// # Safety
/// This function is unsafe because the offset must be valid.
unsafe fn get_memmap_entry(memmap: &LimineMemmapResponse, i: u64) -> &LimineMemmapEntry {
    &*(*memmap.entries.as_ptr().offset(i as isize)).as_ptr()
}

/// Load a PML4 page table to the CR3 register.
///
/// # Arguments
/// `p4_addr` - The address of the page table.
///
/// # Safety
/// The function is unsafe because changing the page table can lead to a memory violation.
pub unsafe fn load_tables_to_cr3(p4_addr: PhysAddr) {
    Cr3::write(
        // UNWRAP: the page frame allocator will make sure that the address will be 4KiB aligned.
        PhysFrame::<Size4KiB>::from_start_address(p4_addr).unwrap(),
        Cr3Flags::empty(),
    );
}

/// Map the kernel's virtual address.
///
/// # Arguments
/// * `pml4` - The page map level 4, the highest page table.
pub fn map_kernel_address(pml4: PhysAddr) {
    let memmap = get_memmap();
    let virtual_address = KERNEL_ADDRESS.get_response().get().unwrap().virtual_base;
    let flags = PageTableFlags::GLOBAL | PageTableFlags::PRESENT;
    let mut entry;
    let mut offset = 0;

    for i in 0..memmap.entry_count {
        // UNSAFE: `i` is between 0 and the entry count.
        entry = unsafe { get_memmap_entry(memmap, i) };

        if entry.typ == LimineMemoryMapEntryType::KernelAndModules {
            while offset < entry.len {
                let physical =
                    PhysFrame::<Size4KiB>::from_start_address(PhysAddr::new(entry.base + offset))
                        .unwrap();

                virtual_memory_manager::map_address(
                    pml4,
                    VirtAddr::new(virtual_address + offset),
                    physical,
                    flags,
                );
                offset += Size4KiB::SIZE;
            }
            break;
        }
    }
}
