use x86_64::{
    registers,
    structures::paging::{
        page_table::PageTableEntry, PageSize, PageTableFlags, PhysFrame, Size1GiB, Size2MiB,
        Size4KiB,
    },
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
        // Safety: The resulting pointer is always in the table because offset is 9 bits
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
    let mut entry: *mut PageTableEntry = core::ptr::null_mut();
    let tables = match physical_address.size() {
        Size4KiB::SIZE => 4,
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
        // The offset is 9 bits. To get the offset we shift to the left all of the bits we already
        // used so that the 9 bits that we want are the top 9 bits, and then we shift to the right
        // by 55 to place the offset at the lower 9 bits.
        let offset = ((virtual_address.as_u64() << used_bits) >> 55) as isize;

        if page_table == 0 {
            // SAFETY: Entry is not null because pml4 has been asserted to be not null
            unsafe {
                page_table = super::page_allocator::allocate()
                    .expect("No free memory for a page table")
                    .start_address()
                    .as_u64();
                // Update the previous entry
                (*entry).set_addr(
                    PhysAddr::new(page_table),
                    PageTableFlags::PRESENT
                        | PageTableFlags::PRESENT
                        | PageTableFlags::USER_ACCESSIBLE,
                );
            }
        }

        // SAFETY: The resulting pointer is always in the table because the offset is 9 bits
        entry = unsafe {
            let entry_virtual =
                ((page_table as *const u64).offset(offset) as u64) + super::HHDM_OFFSET;

            entry_virtual as *mut PageTableEntry
        };
        // Get the physical address from the page table entry
        // SAFETY: entry is not null because it points to a valid location in the page table
        page_table = unsafe { (*entry).addr().as_u64() };
        // Mark the bits of the offset as used
        used_bits += 9;
    }
    // SAFETY: entry is not null because the loop is guarenteed to be ran at least once
    unsafe {
        assert!((*entry).is_unused());
        (*entry).set_addr(physical_address.start_address(), flags);
    }
}
