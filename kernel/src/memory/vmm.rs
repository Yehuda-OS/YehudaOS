use core::fmt;

use x86_64::{
    registers,
    structures::paging::{
        page_table::PageTableEntry, PageSize, PageTableFlags, PhysFrame, Size1GiB, Size2MiB,
        Size4KiB,
    },
    PhysAddr, VirtAddr,
};

const PAGE_TABLE_ENTRIES: u64 = 512;
const PAGE_TABLE_LEVELS: u8 = 4;

#[derive(Debug)]
pub enum MapError {
    /// Not enough memory for a page table.
    OutOfMemory,
    /// `pml4` is 0
    NullPageTable,
    /// The physical frame is 4KiB but the `HUGE_PAGE` flag is set.
    InvalidHugePageFlag,
    /// The physical frame is 2MiB or 1GiB but `flags` does not contain the `HUGE_PAGE` flag.
    MissingHugePageFlag,
    /// The virtual address is already in use.
    EntryAlreadyUsed,
}

#[derive(Debug)]
pub enum UnmapError {
    NullPageTable,
    EntryUnused,
}

impl fmt::Display for MapError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MapError::OutOfMemory => write!(f, "not enough memory for a page table"),
            MapError::NullPageTable => write!(f, "the provided page table is null"),
            MapError::InvalidHugePageFlag => write!(
                f,
                "the physical frame is 4KiB but the huge page flag is set"
            ),
            MapError::MissingHugePageFlag => write!(
                f,
                "the physical frame is 2MiB or 1GiB but the huge page flag is not set"
            ),
            MapError::EntryAlreadyUsed => write!(f, "the virtual address is already in use"),
        }
    }
}

impl fmt::Display for UnmapError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            UnmapError::NullPageTable => write!(f, "{}", MapError::NullPageTable),
            UnmapError::EntryUnused => write!(f, "the virtual address is unused"),
        }
    }
}

/// Get an entry in a page table.
///
/// # Arguments
/// * `page_table` - The physical address of the page table.
/// * `offset` - The offset in the page table.
///
/// # Safety
/// This function is unsafe because `page_table` must be a valid page table and `offset` must be
/// equals or greater than 0 and must be less than 512.
unsafe fn get_page_table_entry(page_table: PhysAddr, offset: u64) -> *mut PageTableEntry {
    let entry_physical = (page_table.as_u64() as *const u64).add(offset as usize) as u64;
    let entry_virtual = entry_physical + super::HHDM_OFFSET;

    entry_virtual as *mut PageTableEntry
}

/// Allocate a page for a page table and set all of its entries to 0.
///
/// # Returns
/// The physical address of the page table or `None` if there is no free memory for the page table.
pub fn create_page_table() -> Option<PhysAddr> {
    let page_table = super::page_allocator::allocate()?.start_address();

    for i in 0..PAGE_TABLE_ENTRIES {
        // SAFETY: the page table was allocated and the offset is in the page table range.
        unsafe {
            (*get_page_table_entry(page_table, i)).set_unused();
        }
    }

    return Some(page_table);
}

/// Walk over all the used page table entries.
/// Does not support huge pages.
/// 
/// # Arguments
/// - `pml4` - The page table to walk over.
/// - `handler` - A callback function that will be called on each used entry.
/// It's parameters are the virtual address of the entry and the physical address
/// that it is mapped to.
pub fn page_table_walker(pml4: PhysAddr, handler: &dyn Fn(VirtAddr, PhysAddr)) {
    let mut p3;
    let mut p2;
    let mut p1;
    let mut entry;
    let mut virtual_address;
    let mut indexes;

    for p4_index in 0..PAGE_TABLE_ENTRIES {
        entry = unsafe { &mut *get_page_table_entry(pml4, p4_index) };
        if entry.is_unused() {
            continue;
        }
        p3 = entry.addr();
        for p3_index in 0..PAGE_TABLE_ENTRIES {
            entry = unsafe { &mut *get_page_table_entry(p3, p3_index) };
            if entry.is_unused() || entry.flags().contains(PageTableFlags::HUGE_PAGE) {
                continue;
            }
            p2 = entry.addr();
            for p2_index in 0..PAGE_TABLE_ENTRIES {
                entry = unsafe { &mut *get_page_table_entry(p2, p2_index) };
                if entry.is_unused() || entry.flags().contains(PageTableFlags::HUGE_PAGE) {
                    continue;
                }
                p1 = entry.addr();
                for p1_index in 0..PAGE_TABLE_ENTRIES {
                    entry = unsafe { &mut *get_page_table_entry(p1, p1_index) };
                    if entry.is_unused() || entry.flags().contains(PageTableFlags::HUGE_PAGE) {
                        continue;
                    }
                    indexes = [p4_index, p3_index, p2_index, p1_index];
                    virtual_address = 0;
                    for index in indexes {
                        // Every index is 9 bits
                        virtual_address |= index;
                        virtual_address <<= 9;
                    }
                    // The offset in the page is 12 bits.
                    virtual_address <<= 12 - 9;
                    handler(VirtAddr::new(virtual_address), entry.addr());
                }
            }
        }
    }
}

/// Returns the physical addresses a virtual address is mapped to or an error if `pml4`
/// is null or the virtual address is unused.
///
/// # Arguments
/// - `pml4` - The page map level 4, the highest page table.
/// - `virtual_address` - The virtual address to translate.
pub fn virtual_to_physical(
    pml4: PhysAddr,
    virtual_address: VirtAddr,
) -> Result<PhysAddr, UnmapError> {
    let mut page_table = pml4.as_u64();
    let mut used_bits = 16; // The highest 16 bits are unused

    if pml4.is_null() {
        return Err(UnmapError::NullPageTable);
    }

    for _ in 0..PAGE_TABLE_LEVELS {
        // The offset is 9 bits. To get the offset we shift to the left all of the bits we already
        // used so that the 9 bits that we want are the top 9 bits, and then we shift to the right
        // by 55 to place the offset at the lower 9 bits.
        let offset = ((virtual_address.as_u64() << used_bits) >> 55);
        // SAFETY: the offset is valid because it is 9 bits.
        let entry = unsafe { &*get_page_table_entry(PhysAddr::new(page_table), offset) };
        let entry_flags = entry.flags();

        if entry.is_unused() {
            return Err(UnmapError::EntryUnused);
        }

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
    Ok(PhysAddr::new(
        page_table + (virtual_address.as_u64() & (!0 >> used_bits)),
    ))
}

/// Maps a virtual address to a physical address.
///
/// # Arguments
/// - `pml4` - The address of the Page Map Level 4.
/// - `virtual_address` - The virtual address to map.
/// - `physical_address` - The physical frame to map the virtual address to.
/// The function supports 2MiB and 1GiB pages.
/// - `flags` - The flags of the last entry.
pub fn map_address<T: PageSize>(
    pml4: PhysAddr,
    virtual_address: VirtAddr,
    physical_address: PhysFrame<T>,
    flags: PageTableFlags,
) -> Result<(), MapError> {
    let mut page_table = pml4.as_u64();
    let mut used_bits = 16; // The highest 16 bits are unused
    let mut entry: *mut PageTableEntry = core::ptr::null_mut();
    let tables = match physical_address.size() {
        Size4KiB::SIZE => {
            if flags.contains(PageTableFlags::HUGE_PAGE) {
                Err(MapError::InvalidHugePageFlag)
            } else {
                Ok(4)
            }
        }
        Size2MiB::SIZE => {
            if flags.contains(PageTableFlags::HUGE_PAGE) {
                Ok(3) // When the page size is 2MiB we stop iterating at p2 table
            } else {
                Err(MapError::MissingHugePageFlag)
            }
        }
        Size1GiB::SIZE => {
            if flags.contains(PageTableFlags::HUGE_PAGE) {
                Ok(2) // When the apge size is 1GiB we stop iterating at p3 table
            } else {
                Err(MapError::MissingHugePageFlag)
            }
        }
        _ => Ok(0), // size always returns one of the above
    }?;

    if pml4.is_null() {
        return Err(MapError::NullPageTable);
    }

    for _ in 0..tables {
        // The offset is 9 bits. To get the offset we shift to the left all the bits we already
        // used so that the 9 bits that we want are the top 9 bits, and then we shift to the right
        // by 55 to place the offset at the lower 9 bits.
        let offset = (virtual_address.as_u64() << used_bits) >> 55;

        if page_table == 0 {
            // SAFETY: Entry is not null because pml4 has been asserted to be not null
            unsafe {
                page_table = create_page_table().ok_or(MapError::OutOfMemory)?.as_u64();
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
        if (*entry).is_unused() {
            (*entry).set_addr(physical_address.start_address(), flags);
        } else {
            return Err(MapError::EntryAlreadyUsed);
        }
    }

    Ok(())
}

/// Get a page table a virtual address is using.
///
/// # Arguments
/// * `virtual_address` - The virtual address to translate.
/// * `level` - the level of the page table.
fn virt_addr_to_page_table(level: u8, virtual_address: VirtAddr) -> PhysAddr {
    let mut page_table = registers::control::Cr3::read().0.start_address().as_u64();
    let mut used_bits = 16; // The highest 16 bits are unused

    for _ in 0..(PAGE_TABLE_LEVELS - level) {
        // The offset is 9 bits. To get the offset we shift to the left all of the bits we already
        // used so that the 9 bits that we want are the top 9 bits, and then we shift to the right
        // by 55 to place the offset at the lower 9 bits.
        let offset = (virtual_address.as_u64() << used_bits) >> 55;
        // SAFETY: the offset is valid because it is 9 bits.
        let entry = unsafe { &*get_page_table_entry(PhysAddr::new(page_table), offset) };

        // Get the physical address from the page table entry
        page_table = entry.addr().as_u64();
        // Mark the bits of the offset as used
        used_bits += 9;
    }

    return PhysAddr::new(page_table);
}

/// check if the page table is free
///
/// # Arguments
/// * `table_addr` - the address of the page table.
fn is_page_table_free(table_addr: &PhysAddr) -> bool {
    let page_table: u64 = table_addr.as_u64();
    let mut entry;

    for i in 0..PAGE_TABLE_ENTRIES {
        // SAFETY: the offset is valid because it is 9 bits.
        entry = unsafe { get_page_table_entry(PhysAddr::new(page_table), i) };

        // if entry is used, return false
        if !unsafe { (*entry).is_unused() } {
            return false;
        }
    }

    true
}

/// Unmap a virtual address.
///
/// # Arguments
/// * `pml4` - The address of the Page Map Level 4.
/// * `virtual_address` - The virtual address to unmap.
/// ### panics if:
/// - `pml4` is 0.
/// - The virtual address is already unused.
pub fn unmap_address(pml4: PhysAddr, virtual_address: VirtAddr) -> Result<(), UnmapError> {
    let mut page_table = pml4.as_u64();
    let mut used_bits = 16; // The highest 16 bits are unused
    let mut entry: *mut PageTableEntry = core::ptr::null_mut();
    let mut level_counter: u8 = 0;

    if pml4.is_null() {
        return Err(UnmapError::NullPageTable);
    }

    for _ in 0..PAGE_TABLE_LEVELS {
        let offset = (virtual_address.as_u64() << used_bits) >> 55;
        // SAFETY: the offset is valid because it is 9 bits.
        entry = unsafe { get_page_table_entry(PhysAddr::new(page_table), offset) };

        // Get the physical address from the page table entry
        page_table = unsafe { (*entry).addr().as_u64() };
        // Mark the bits of the offset as used
        used_bits += 9;

        level_counter += 1;

        // If the huge page flag is on, that means that this was the last page table
        if unsafe { (*entry).flags() }.contains(PageTableFlags::HUGE_PAGE) {
            break;
        }
    }

    // SAFETY: `entry` is not null because the loop is guarenteed to be ran at least once.
    unsafe {
        if (*entry).is_unused() {
            return Err(UnmapError::EntryUnused);
        }
        (*entry).set_unused();
    };

    for i in 1..level_counter {
        let table = virt_addr_to_page_table(i, VirtAddr::new(page_table));

        if is_page_table_free(&table) {
            unsafe { super::page_allocator::free(PhysFrame::from_start_address(table).unwrap()) };
        } else {
            break;
        }
    }

    Ok(())
}
