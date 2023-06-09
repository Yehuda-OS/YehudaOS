pub mod allocator;
pub mod page_allocator;
pub mod vmm;

use limine::{
    LimineMemmapEntry, LimineMemmapRequest, LimineMemmapResponse, LimineMemoryMapEntryType,
};
use x86_64::{
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{PageSize, PageTableFlags, PhysFrame, Size1GiB, Size2MiB, Size4KiB},
    PhysAddr, VirtAddr,
};

pub const KERNEL_ADDRESS: u64 = 0xffff_ffff_8000_0000;
pub const HHDM_OFFSET: u64 = 0xffff_8000_0000_0000;

pub static MEMMAP: LimineMemmapRequest = LimineMemmapRequest::new(0);
pub static mut PAGE_TABLE: PhysAddr = PhysAddr::zero();

/// Returns the kernel's page table.
pub fn get_page_table() -> PhysAddr {
    unsafe { PAGE_TABLE }
}

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

/// Flush the TLB cache.
pub fn flush_tlb_cache() {
    unsafe {
        load_tables_to_cr3(Cr3::read().0.start_address());
    }
}

fn get_last_phys_addr() -> u64 {
    let last_entry = unsafe { get_memmap_entry(get_memmap(), get_memmap().entry_count - 1) };

    last_entry.base + last_entry.len
}

/// Map a memmap entry to a virtual address.
///
/// # Arguments
/// - `virtual_addr` - The required virtual start address.
/// - `entry` - The entry to map.
/// - `flags` - The page table flags to use.
fn map_memmap_entry(
    virtual_addr: VirtAddr,
    entry: &LimineMemmapEntry,
    flags: PageTableFlags,
) -> Result<(), vmm::MapError> {
    let mut offset = 0;
    let mut physical;

    while offset < entry.len {
        physical = PhysAddr::new(entry.base + offset);

        vmm::map_address(
            unsafe { PAGE_TABLE },
            VirtAddr::new(virtual_addr.as_u64() + offset),
            PhysFrame::<Size4KiB>::from_start_address(physical).unwrap(),
            flags,
        )?;
        offset += Size4KiB::SIZE;
    }

    Ok(())
}

/// Map the kernel's virtual address.
pub fn map_kernel_address() -> Result<(), vmm::MapError> {
    let memmap = get_memmap();
    let flags = PageTableFlags::GLOBAL | PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    let mut entry;

    for i in 0..memmap.entry_count {
        // UNSAFE: `i` is between 0 and the entry count.
        entry = unsafe { get_memmap_entry(memmap, i) };

        if entry.typ == LimineMemoryMapEntryType::KernelAndModules {
            map_memmap_entry(VirtAddr::new(KERNEL_ADDRESS), entry, flags)?;

            break;
        }
    }

    Ok(())
}

/// Map every physical address to virtual address using hhdm.
///
/// # Arguments
/// * `pml4` - The page map level 4, the highest page table.
pub fn create_hhdm(pml4: PhysAddr) -> Result<(), vmm::MapError> {
    let last_addr = get_last_phys_addr();
    let flags = PageTableFlags::GLOBAL | PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    let mut offset: u64 = 0;

    while offset < last_addr {
        let physical = PhysAddr::new(offset);

        if last_addr - physical.as_u64() >= Size1GiB::SIZE {
            vmm::map_address(
                pml4,
                VirtAddr::new(HHDM_OFFSET + offset),
                PhysFrame::<Size1GiB>::from_start_address(physical).unwrap(),
                flags | PageTableFlags::HUGE_PAGE,
            )?;

            offset += Size1GiB::SIZE;
        } else if last_addr - physical.as_u64() >= Size2MiB::SIZE {
            vmm::map_address(
                pml4,
                VirtAddr::new(HHDM_OFFSET + offset),
                PhysFrame::<Size2MiB>::from_start_address(physical).unwrap(),
                flags | PageTableFlags::HUGE_PAGE,
            )?;

            offset += Size2MiB::SIZE;
        } else {
            vmm::map_address(
                pml4,
                VirtAddr::new(HHDM_OFFSET + offset),
                PhysFrame::<Size4KiB>::from_start_address(physical).unwrap(),
                flags,
            )?;

            offset += Size4KiB::SIZE;
        }
    }

    Ok(())
}

/// Identity map the framebuffer and any bootloader reclaimable memory that does not contain the
/// page tables and the stack.
pub fn map_bootloader_memory() -> Result<(), vmm::MapError> {
    let memmap = get_memmap();
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    let mut entry;
    let mut rsp;

    unsafe { core::arch::asm!("mov {rsp}, rsp", rsp=out(reg)rsp) };

    for i in 0..memmap.entry_count {
        entry = unsafe { get_memmap_entry(memmap, i) };

        if entry.typ == LimineMemoryMapEntryType::Framebuffer {
            map_memmap_entry(VirtAddr::new(entry.base), entry, flags)?;
        } else if entry.typ == LimineMemoryMapEntryType::BootloaderReclaimable {
            if entry.base > rsp || entry.base + entry.len < rsp {
                map_memmap_entry(VirtAddr::new(entry.base), entry, flags)?;
            }
        }
    }

    Ok(())
}
