use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};

use x86_64::{
    structures::paging::{PageSize, Size4KiB},
    VirtAddr,
};

pub const HEAP_START: u64 = 0x_4444_4444_0000;
pub const MAX_PAGES: u64 = 25; // 100 KiB

#[global_allocator]
static ALLOCATOR: Locked<Allocator> = Locked::<Allocator>::new(Allocator::new(HEAP_START));

pub struct Allocator {
    heap_start: u64,
    pages: u64,
}

pub struct HeapBlock {
    size: u64,
    prev: *mut HeapBlock,
}

impl Allocator {
    pub const fn new(heap_start: u64) -> Self {
        Allocator {
            heap_start,
            pages: 0,
        }
    }
}

impl HeapBlock {
    const FREE_BIT: u8 = 63;
    const HAS_NEXT_BIT: u8 = 62;

    pub const fn empty() -> Self {
        HeapBlock {
            size: 0,
            prev: null_mut(),
        }
    }

    pub const fn new(free: bool, has_next: bool, mut size: u64, prev: *mut HeapBlock) -> Self {
        if free {
            size |= 1 << HeapBlock::FREE_BIT;
        }
        if has_next {
            size |= 1 << HeapBlock::HAS_NEXT_BIT;
        }

        HeapBlock { size, prev }
    }

    /// Get the size of the block.
    pub fn size(&self) -> u64 {
        // The two top most bits are used as flags.
        self.size << 2 >> 2
    }

    /// Returns `true` if the block is free.
    pub fn free(&self) -> bool {
        // The top most bit of the size represents if the block is free.
        self.size >> 63 == 1
    }

    // Returns `true` if the block is not the last in the linked list.
    pub fn has_next(&self) -> bool {
        // The second top most bit of the size represents
        // whether the block has another block after it.
        self.size << 1 >> 63 == 1
    }

    pub fn set_has_next(&mut self, has_next: bool) {
        if has_next {
            self.size |= 1 << HeapBlock::HAS_NEXT_BIT;
        }
    }

    /// Returns `true` if the block is the first in the list.
    pub fn has_prev(&self) -> bool {
        self.prev == null_mut()
    }

    // Get the next heap block in the list.
    pub fn next(&self) -> *mut HeapBlock {
        if self.has_next() {
            unsafe {
                let start_of_block = (self as *const HeapBlock).offset(1) as u64;

                (start_of_block + self.size) as *mut HeapBlock
            }
        } else {
            null_mut()
        }
    }

    pub fn get_prev(&self) -> *mut HeapBlock {
        self.prev
    }
}

/// Request pages from the page allocator until there is enough space for the required data size
/// and create a [`HeapBlock`](HeapBlock) instance at the start of the allocated space.
///
/// # Arguments
/// - `allocator` - The allocator instance that is being used.
/// - `size` - The required allocation size.
/// - `align` - The required alignment for the allocation start address.
///
/// # Returns
/// A pointer to the created [`HeapBlock`](HeapBlock) instance.
fn alloc_node(allocator: &Allocator, size: usize, align: usize) -> *mut HeapBlock {
    let start = VirtAddr::new(allocator.heap_start + allocator.pages * Size4KiB::SIZE);
    let mut current_size = 0;

    null_mut()
}

unsafe impl GlobalAlloc for Locked<Allocator> {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        let allocator = self.lock();
        let size = _layout.size();
        let align = _layout.align();
        let start = if allocator.pages == 0 {
            null_mut()
        } else {
            allocator.heap_start as *mut HeapBlock
        };
        let mut curr = start;

        while curr != null_mut() {
            let adjustment = align - (*curr).size() as usize % align;

            if (*curr).free() && (*curr).size() as usize >= size + adjustment {
                break;
            }
            curr = (*curr).next();
        }
        if curr == null_mut() {
            curr = alloc_node(&allocator, size, align);
            if curr == null_mut() {
                return null_mut();
            }
        }

        null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        panic!("dealloc should be never called")
    }
}

/// A wrapper around spin::Mutex to permit trait implementations.
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}
