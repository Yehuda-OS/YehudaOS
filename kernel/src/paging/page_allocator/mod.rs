use x86_64::PhysAddr;

static mut FREE_LIST_START: *mut FreePageNode = core::ptr::null_mut();

struct FreePageNode {
    pub next: *mut FreePageNode,
}

pub unsafe fn free(address: PhysAddr) {
    let free_page =
        (super::HHDM_OFFSET + (address.as_u64() & 0xffff_ffff_ffff_f000)) as *mut FreePageNode;

    *free_page = FreePageNode {
        next: FREE_LIST_START,
    };
    FREE_LIST_START = free_page;
}

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
