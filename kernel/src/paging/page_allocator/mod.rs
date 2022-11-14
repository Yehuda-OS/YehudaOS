use limine::LimineMemoryMapEntryType;
use x86_64::{
    structures::paging::{PageSize, PhysFrame, Size4KiB},
    PhysAddr,
};

static mut FREE_LIST_START: *mut FreePageNode = core::ptr::null_mut();

struct FreePageNode {
    pub next: *mut FreePageNode,
}

/// Returns the address of a newly allocated physical page, or None if there are no free pages.
pub unsafe fn allocate() -> Option<PhysFrame> {
    let free_page;

    if FREE_LIST_START.is_null() {
        return None;
    } else {
        free_page = PhysFrame::from_start_address(PhysAddr::new(
            FREE_LIST_START as u64 - super::HHDM_OFFSET,
        ))
        .unwrap();
        FREE_LIST_START = (*FREE_LIST_START).next;
    }

    return Some(free_page);
}

/// Free a physical page that was previously allocated with `allocate`.
///
/// # Arguments
/// * address - Physical address of the page.
pub unsafe fn free(address: PhysFrame) {
    let free_page = (super::HHDM_OFFSET + address.start_address().as_u64()) as *mut FreePageNode;

    *free_page = FreePageNode {
        next: FREE_LIST_START,
    };
    FREE_LIST_START = free_page;
}

fn mark_free_memory() {
    let memmap = super::MEMMAP.get_response().get().unwrap();

    for i in 0..memmap.entry_count {
        let entry = unsafe { &*(*memmap.entries.as_ptr().offset(i as isize)).as_ptr() };
        let mut current;

        if entry.typ == LimineMemoryMapEntryType::Usable {
            current = entry.base;
            while current + Size4KiB::SIZE <= entry.base + entry.len {
                unsafe {
                    // UNWRAP: usable entries are 4KiB aligned
                    free(PhysFrame::from_start_address(PhysAddr::new(current)).unwrap())
                }
                current += 0x1000;
            }
        }
    }
}
