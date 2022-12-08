use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

#[global_allocator]
static ALLOCATOR: Locked<Allocator> = Locked::<Allocator>::new(Allocator::new());

pub struct Allocator;
struct HeapBlock {
    size: u64,
    prev: *mut HeapBlock,
}

impl Allocator {
    pub const fn new() -> Self {
        Allocator
    }
}

impl HeapBlock {
    pub const fn new() -> Self {
        HeapBlock {
            size: 0,
            prev: null_mut(),
        }
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

    /// Returns `true` if the block is the first in the list.
    pub fn has_prev(&self) -> bool {
        self.prev == null_mut()
    }

    // Get the next heap block in the list.
    pub fn get_next(&self) -> *mut HeapBlock {
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

unsafe impl GlobalAlloc for Locked<Allocator> {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
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
