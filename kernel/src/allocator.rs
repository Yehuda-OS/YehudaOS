use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};

use x86_64::{
    structures::paging::{PageSize, PageTableFlags, Size4KiB},
    PhysAddr, VirtAddr,
};

pub const HEAP_START: u64 = 0x_4444_4444_0000;
pub const MAX_PAGES: u64 = 25; // 100 KiB

const HEADER_SIZE: usize = core::mem::size_of::<HeapBlock>();

#[global_allocator]
static ALLOCATOR: Locked<Allocator> =
    Locked::<Allocator>::new(Allocator::new(HEAP_START, unsafe {
        super::paging::PAGE_TABLE
    }));

pub struct Allocator {
    heap_start: u64,
    pages: u64,
    page_table: PhysAddr,
}

pub struct HeapBlock {
    size: u64,
    prev: *mut HeapBlock,
}

impl Allocator {
    pub const fn new(heap_start: u64, page_table: PhysAddr) -> Self {
        Allocator {
            heap_start,
            pages: 0,
            page_table,
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
        self.size >> HeapBlock::FREE_BIT == 1
    }

    pub fn set_free(&mut self, free: bool) {
        if free {
            self.size |= 1 << HeapBlock::FREE_BIT;
        }
        else {
            self.size &= !(1 << HeapBlock::FREE_BIT);
        }
    }

    // Returns `true` if the block is not the last in the linked list.
    pub fn has_next(&self) -> bool {
        // The second top most bit of the size represents
        // whether the block has another block after it.
        self.size & (1 << HeapBlock::HAS_NEXT_BIT) >> HeapBlock::HAS_NEXT_BIT == 1
    }

    pub fn set_has_next(&mut self, has_next: bool) {
        if has_next {
            self.size |= 1 << HeapBlock::HAS_NEXT_BIT;
        }
        else {
            self.size &= !(1 << HeapBlock::HAS_NEXT_BIT);
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

/// Returns the required adjustment of a data block to match the required allocation alignment.
///
/// # Arguments
/// - `addr` - Pointer to the heap block.
/// - `align` - The required alignment.
fn get_adjustment(addr: *mut HeapBlock, align: usize) -> usize {
    let data_start_address = unsafe { addr.offset(1) } as usize;

    align - data_start_address % align
}

/// Request pages from the page allocator until there is enough space for the required data size
/// and create a `HeapBlock` instance at the start of the allocated space.
///
/// # Arguments
/// - `allocator` - The `Allocator` instance that is being used.
/// - `size` - The required allocation size.
/// - `align` - The required alignment for the allocation's start address.
///
/// # Returns
/// A pointer to the created `HeapBlock`, or [`None`] if the allocation failed.
fn alloc_node(
    allocator: &mut Allocator,
    last: *mut HeapBlock,
    size: usize,
    align: usize,
) -> Option<*mut HeapBlock> {
    let start = VirtAddr::new(allocator.heap_start + allocator.pages * Size4KiB::SIZE);
    let mut current_size = 0;
    let adjustment = get_adjustment(start.as_mut_ptr(), align);
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    let allocated;

    while current_size < size + adjustment {
        if let Some(page) = super::paging::page_allocator::allocate() {
            allocator.pages += 1;
            current_size += Size4KiB::SIZE as usize;
            super::paging::virtual_memory_manager::map_address(
                allocator.page_table,
                start + current_size,
                page,
                flags,
            );
        } else {
            return None;
        }
    }
    allocated = start.as_mut_ptr::<HeapBlock>();
    unsafe {
        (*last).set_has_next(true);
        (*allocated) = HeapBlock::new(true, false, (current_size - HEADER_SIZE) as u64, last);
    };

    Some(allocated)
}

/// Returns a usable heap block for a specific allocation request
/// or [`None`] if the allocation fails.
///
/// # Arguments
/// - `allocator` - The `Allocator` instance that is being used.
/// - `size` - The required allocation size.
/// - `align` - The required alignment for the allocation's start address.
///
/// # Safety
/// This function is unsafe because the heap must not be corrupted.
unsafe fn find_usable_block(
    allocator: &mut Allocator,
    size: usize,
    align: usize,
) -> Option<*mut HeapBlock> {
    let start = if allocator.pages == 0 {
        null_mut()
    } else {
        allocator.heap_start as *mut HeapBlock
    };
    let mut curr = start;

    loop {
        let curr_adjustment = get_adjustment(curr, align);

        if curr == null_mut() || !(*curr).has_next() {
            return if let Some(allocated) = alloc_node(allocator, curr, size, align) {
                Some(allocated)
            } else {
                None
            };
        } else if (*curr).free() && (*curr).size() as usize >= size + curr_adjustment {
            return Some(curr);
        }
        curr = (*curr).next();
    }
}

fn merge_blocks(block: *mut HeapBlock) {}

fn shrink_block(block: *mut HeapBlock, size: usize) {}

fn resize_block(block: *mut HeapBlock, size: usize, align: usize) -> *mut HeapBlock {
    null_mut()
}

unsafe impl GlobalAlloc for Locked<Allocator> {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();
        let size = _layout.size();
        let align = _layout.align();
        let block = find_usable_block(&mut allocator, size, align);

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
