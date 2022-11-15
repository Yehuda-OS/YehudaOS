use x86_64::PhysAddr;
use limine::{LimineMemoryMapEntryType};
use super::PAGE_SIZE;

static mut FREE_LIST_START: *mut FreePageNode = core::ptr::null_mut();

struct FreePageNode {
    pub next: *mut FreePageNode,
}

/// Returns the address of a newly allocated physical page.
pub unsafe fn allocate() -> PhysAddr {
    let free_page;

    if FREE_LIST_START.is_null() {
        free_page = PhysAddr::zero();
    } else {
        free_page = PhysAddr::new(FREE_LIST_START as u64 - super::HHDM_OFFSET);
        FREE_LIST_START = (*FREE_LIST_START).next;
    }

    return free_page;
}

/// Free a physical page that was previously allocated with `allocate`.
/// 
/// # Arguments
/// * address - Physical address of the page.
pub unsafe fn free(address: PhysAddr) {
    let free_page =
        (super::HHDM_OFFSET + (address.as_u64() & 0xffff_ffff_ffff_f000)) as *mut FreePageNode;

    *free_page = FreePageNode {
        next: FREE_LIST_START,
    };
    FREE_LIST_START = free_page;
}

fn mark_free_memory() {
    let memmap = super::MEMMAP.get_response().get().unwrap(); 

    for i in 0..memmap.entry_count {
        let entry = unsafe {
            &*(*memmap.entries.as_ptr().offset(i as isize))
                .as_ptr()
        };
        let mut current;

        if entry.typ == LimineMemoryMapEntryType::Usable {
            current = entry.base;
            while current + PAGE_SIZE <= entry.base + entry.len {
                unsafe {
                    free(PhysAddr::new(current))
                }
                current += 0x1000;
            }
        }
    }
}
