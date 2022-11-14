use x86_64::{
    registers,
    structures::paging::{page_table::PageTableEntry, PageTableFlags, PhysFrame},
    PhysAddr, VirtAddr,
};

/// Get the physical addresses a virtual address is mapped to.
///
/// # Arguments
/// * `virtual_address` - The virtual address to translate.
/// * `hhdm_offset` - The offset of the higher half direct map.
pub fn virtual_to_physical(virtual_address: VirtAddr) -> PhysAddr {
    let mut page_table = registers::control::Cr3::read().0.start_address().as_u64();
    let mut used_bits = 16; // The highest 16 bits are unused

    // Iterate 4 times because there are 4 page tables
    for _ in 0..4 {
        // The offset is 9 bits. To get the offset we shift to the left all of the bits we already
        // used so that the 9 bits that we want are the top 9 bits, and then we shift to the right
        // by 55 to place the offset at the lower 9 bits.
        let offset = ((virtual_address.as_u64() << used_bits) >> 55) as isize;
        let entry_bits = unsafe {
            let entry_virtual =
                ((page_table as *const u64).offset(offset) as u64) + super::HHDM_OFFSET;

            *(entry_virtual as *const u64)
        };
        let entry_flags = PageTableFlags::from_bits_truncate(entry_bits);

        // Get the physical address from the page table entry
        page_table = entry_bits & 0x000f_ffff_ffff_f000;
        // Mark the bits of the offset as used
        used_bits += 9;
        // If the huge page flag is on, that means that this was the last page table
        // and the next address is the physical page
        if entry_flags.contains(PageTableFlags::HUGE_PAGE) {
            break;
        }
    }

    // Use all the unused bits as the offset in the physical page
    return PhysAddr::new(
        page_table + (virtual_address.as_u64() & (0xffff_ffff_ffff_ffff >> used_bits)),
    );
}

pub fn map_address(
    pml4: PhysAddr,
    virtual_address: VirtAddr,
    physical_address: PhysFrame,
    flags: PageTableFlags,
) {
    let mut page_table = pml4.as_u64();
    let mut used_bits = 16; // The highest 16 bits are unused
    let mut entry = core::ptr::null_mut();

    // Iterate up to 4 times because there are 4 page tables
    // and we want the entry in the last table
    // If the size of the physical frame divided by is 2MiB or 1GiB,
    // after dividing it by 512 it will produce the required amount of
    // iterations because 4KiB * 512 * 512 = 2MiB * 512 = 1GiB
    for _ in (physical_address.size() / super::PAGE_SIZE / 512)..4 {
        // The offset is 9 bits. To get the offset we shift to the left all of the bits we already
        // used so that the 9 bits that we want are the top 9 bits, and then we shift to the right
        // by 55 to place the offset at the lower 9 bits.
        let offset = ((virtual_address.as_u64() << used_bits) >> 55) as isize;

        entry = unsafe {
            let entry_virtual =
                ((page_table as *const u64).offset(offset) as u64) + super::HHDM_OFFSET;

            entry_virtual as *mut PageTableEntry
        };
        // Get the physical address from the page table entry
        page_table = unsafe { (*entry).addr().as_u64() };
        // Mark the bits of the offset as used
        used_bits += 9;
    }
    unsafe {
        (*entry).set_addr(physical_address.start_address(), flags);
    }
}
