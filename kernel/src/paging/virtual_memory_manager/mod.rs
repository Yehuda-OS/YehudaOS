use x86_64::{
    registers,
    structures::paging::{
        page_table::PageTableEntry, PageSize, PageTableFlags, PhysFrame, Size1GiB, Size2MiB,
        Size4KiB,
    },
    PhysAddr, VirtAddr,
};

/// Get an entry in a page table.
///
/// # Arguments
/// * `page_table` - The physical address of the page table.
/// * `offset` - The offset in the page table.
///
/// # Safety
/// This function is unsafe because `page_table` must be a valid page table and `offset` must be
/// equals or greater than 0 and must be less than 512.
unsafe fn get_page_table_entry(page_table: PhysAddr, offset: isize) -> *mut PageTableEntry {
    let entry_physical = (page_table.as_u64() as *const u64).offset(offset) as u64;
    let entry_virtual = entry_physical + super::HHDM_OFFSET;

    entry_virtual as *mut PageTableEntry
}

/// Allocate a page for a page table and set all of its entries to 0.
/// Panics if there is no free memory for the page table.
///
/// # Returns
/// The physical address of the page table.
pub fn create_page_table() -> PhysAddr {
    let page_table = super::page_allocator::allocate()
        .expect("No free memory for a page table")
        .start_address();

    for i in 0..super::PAGE_TABLE_ENTRIES {
        // SAFETY: the page table was allocated and the offset is in the page table range.
        unsafe {
            (*get_page_table_entry(page_table, i)).set_unused();
        }
    }

    return page_table;
}

/// Returns the physical addresses a virtual address is mapped to.
///
/// # Arguments
/// * `pml4` - The page map level 4, the highest page table.
/// * `virtual_address` - The virtual address to translate.
pub fn virtual_to_physical(pml4: PhysAddr, virtual_address: VirtAddr) -> PhysAddr {
    let mut page_table = pml4.as_u64();
    let mut used_bits = 16; // The highest 16 bits are unused

    // Iterate 4 times because there are 4 page tables
    for _ in 0..4 {
        // The offset is 9 bits. To get the offset we shift to the left all of the bits we already
        // used so that the 9 bits that we want are the top 9 bits, and then we shift to the right
        // by 55 to place the offset at the lower 9 bits.
        let offset = ((virtual_address.as_u64() << used_bits) >> 55) as isize;
        // SAFETY: the offset is valid because it is 9 bits.
        let entry = unsafe { &*get_page_table_entry(PhysAddr::new(page_table), offset) };
        let entry_flags = entry.flags();

        // Get the physical address from the page table entry
        page_table = entry.addr().as_u64();
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

/// Maps a virtual address to a physical address.
///
/// # Arguments
/// * `pml4` - The address of the Page Map Level 4.
/// * `virtual_address` - The virtual address to map.
/// * `physical_address` - The physical frame to map the virtual address to.
/// The function supports 2MiB and 1GiB pages.
/// * `flags` - The flags of the last entry.
///
/// ### Panics if:
/// - `pml4` is 0.
/// - The virtual address is already in use.
/// - The physical frame is 4KiB but the `HUGE_PAGE` flag is set.
/// - The physical frame is 2MiB or 1GiB but `flags` does not contain the `HUGE_PAGE` flag.
pub fn map_address<T: PageSize>(
    pml4: PhysAddr,
    virtual_address: VirtAddr,
    physical_address: PhysFrame<T>,
    flags: PageTableFlags,
) {
    let mut page_table = pml4.as_u64();
    let mut used_bits = 16; // The highest 16 bits are unused
    let mut entry: *mut PageTableEntry = core::ptr::null_mut();
    let tables = match physical_address.size() {
        Size4KiB::SIZE => {
            assert!(
                !flags.contains(PageTableFlags::HUGE_PAGE),
                "Huge page flag on 4KiB page"
            );
            4
        }
        Size2MiB::SIZE => {
            assert!(
                flags.contains(PageTableFlags::HUGE_PAGE),
                "Missing huge page flag"
            );

            3 // When the page size is 2MiB we stop iterating at p2 table
        }
        Size1GiB::SIZE => {
            assert!(
                flags.contains(PageTableFlags::HUGE_PAGE),
                "Missing huge page flag"
            );

            2 // When the apge size is 1GiB we stop iterating at p3 table
        }
        _ => 0, // size always returns one of the above
    };

    assert!(!pml4.is_null(), "Invalid page table: address 0 was given");

    for _ in 0..tables {
        // The offset is 9 bits. To get the offset we shift to the left all the bits we already
        // used so that the 9 bits that we want are the top 9 bits, and then we shift to the right
        // by 55 to place the offset at the lower 9 bits.
        let offset = ((virtual_address.as_u64() << used_bits) >> 55) as isize;

        if page_table == 0 {
            // SAFETY: Entry is not null because pml4 has been asserted to be not null
            unsafe {
                page_table = create_page_table().as_u64();
                // Update the previous entry
                (*entry).set_addr(
                    PhysAddr::new(page_table),
                    PageTableFlags::PRESENT
                        | PageTableFlags::WRITABLE
                        | PageTableFlags::USER_ACCESSIBLE,
                );
            }
        }

        // SAFETY: The offset is valid because it is 9 bits
        entry = unsafe { get_page_table_entry(PhysAddr::new(page_table), offset) };
        // Get the physical address from the page table entry.
        // SAFETY: `entry` is not null because it points to a valid location in the page table.
        page_table = unsafe { (*entry).addr().as_u64() };
        // Mark the bits of the offset as used
        used_bits += 9;
    }
    // SAFETY: `entry` is not null because the loop is guarenteed to be ran at least once.
    unsafe {
        assert!((*entry).is_unused(), "Virtual address is already in use");
        (*entry).set_addr(physical_address.start_address(), flags);
    }
}
