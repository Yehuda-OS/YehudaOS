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
pub fn allocate() -> Option<PhysFrame> {
    let free_page;

    // SAFETY: the kernel is not multithreaded.
    if unsafe { FREE_LIST_START.is_null() } {
        return None;
    } else {
        // SAFETY: the kernel is not multithreaded.
        free_page = unsafe {
            PhysFrame::from_start_address(PhysAddr::new(
                FREE_LIST_START as u64 - super::HHDM_OFFSET,
            ))
            // UNWRAP: Freed pages are always 4KiB aligned
            .unwrap()
        };
        // SAFETY: if the first free page is invalid a page fault was already triggered.
        unsafe {
            FREE_LIST_START = (*FREE_LIST_START).next;
        };
    }

    return Some(free_page);
}

/// Free a physical page that was previously allocated with `allocate`.
///
/// # Arguments
/// * address - Physical address of the page.
///
/// # Safety
/// The function may produce a page fault if the address is not valid.
pub unsafe fn free(address: PhysFrame) {
    let free_page = (super::HHDM_OFFSET + address.start_address().as_u64()) as *mut FreePageNode;

    *free_page = FreePageNode {
        next: FREE_LIST_START,
    };
    FREE_LIST_START = free_page;
}

/// Initialize the free pages list with the usable pages in limine's memmap and initialize the value
/// of the hhdm offset.
pub fn initialize() {
    let memmap = super::get_memmap();

    for i in 0..memmap.entry_count {
        // UNSAFE: `i` is between 0 and the entry count.
        let entry = unsafe { super::get_memmap_entry(memmap, i) };
        let mut current;

        if entry.typ == LimineMemoryMapEntryType::Usable {
            current = entry.base;
            while current + Size4KiB::SIZE <= entry.base + entry.len {
                unsafe {
                    // UNWRAP: usable entries are 4KiB aligned.
                    free(PhysFrame::from_start_address(PhysAddr::new(current)).unwrap())
                }
                current += Size4KiB::SIZE;
            }
        }
    }
}
