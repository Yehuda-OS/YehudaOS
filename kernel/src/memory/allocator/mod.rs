use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};

use heap_block::HeapBlock;
use x86_64::{
    structures::paging::{PageSize, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::println;

mod heap_block;

pub const HEAP_START: u64 = 0x_4444_4444_0000;
pub const MAX_PAGES: u64 = 25; // 100 KiB

const HEADER_SIZE: usize = core::mem::size_of::<HeapBlock>();

#[global_allocator]
pub static mut ALLOCATOR: Locked<Allocator> =
    Locked::<Allocator>::new(Allocator::new(HEAP_START, unsafe { super::PAGE_TABLE }));

pub struct Allocator {
    heap_start: u64,
    pages: u64,
    page_table: PhysAddr,
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
        if let Some(page) = super::page_allocator::allocate() {
            allocator.pages += 1;
            super::virtual_memory_manager::map_address(
                allocator.page_table,
                start + current_size,
                page,
                flags,
            );
            current_size += Size4KiB::SIZE as usize;
        } else {
            return None;
        }
    }
    allocated = start.as_mut_ptr::<HeapBlock>();
    unsafe {
        if !last.is_null() {
            (*last).set_has_next(true);
        }
        (*allocated) = HeapBlock::new(true, false, (current_size - HEADER_SIZE) as u64, last);
    };

    Some(allocated)
}

/// function that deallocate a node
/// # Arguments
/// - `allocator` - The `Allocator` instance that is being used.
/// - `block` - The block to deallocate.
/// - `size` - the size to deallocate
unsafe fn dealloc_node(allocator: &mut Allocator, block: *mut HeapBlock, size: usize) {
    (*block).set_free(true);
    if (*block).has_next() && (*(*block).next()).free() {
        merge_blocks(block);
        dealloc_node(allocator, block, size);
    }
    if (*block).has_prev() && (*(*block).prev()).free() {
        merge_blocks((*block).prev());
        dealloc_node(allocator, (*block).prev(), size);
    }

    if !(*block).has_next() {
        while (*block).size() > Size4KiB::SIZE {
            crate::memory::page_allocator::free(
                PhysFrame::from_start_address(
                    crate::memory::virtual_memory_manager::virtual_to_physical(
                        allocator.page_table,
                        VirtAddr::new(block.addr() as u64),
                    ),
                )
                .expect("Error: failed to get block physical address"),
            );

            (*block).set_size((*block).size() - Size4KiB::SIZE);
        }
    }
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

        if curr.is_null() || !(*curr).has_next() {
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

/// Merge a block with the next block after it.
///
/// # Arguments
/// - `block` - The block to merge.
///
/// # Safety
/// This function is unsafe because it requires the block to have a free block after it.
unsafe fn merge_blocks(block: *mut HeapBlock) {
    let next = *(*block).next();

    (*block).set_size((*block).size() + next.size());
    (*block).set_has_next(next.has_next());
}

/// Split a block into two blocks, one with the required size and one with the remaining size.
///
/// # Arguments
/// - `block` - The block to shrink.
/// - `size` - The required size of the block, including any alignment adjustments.
///
/// # Safety
/// This function is unsafe because the block must have enough space to contain a `HeapBlock` header
/// for the next block.
unsafe fn shrink_block(block: *mut HeapBlock, size: usize) {
    let has_next = (*block).has_next();
    let extra = (*block).size() as usize - size;

    (*block).set_size(size as u64);
    (*block).set_has_next(true);
    *(*block).next() = HeapBlock::new(true, has_next, (extra - HEADER_SIZE) as u64, block);
}

/// Check if the block is bigger than the required size and if it is resize it accordingly and
/// merge it with the other blocks around it if it is possible.
///
/// # Arguments
/// - `block` - A free block with at least `size` space.
/// - `size` - The required allocation size.
/// - `align` - The required alignment for the allocation's start address.
///
/// # Safety
/// This function is unsafe because the heap must not be corrupted and the block must be valid.
unsafe fn resize_block(mut block: *mut HeapBlock, size: usize, align: usize) -> *mut HeapBlock {
    let mut adjustment = get_adjustment(block, align);

    if (*block).size() as usize > size + adjustment {
        // Check if the current block can be merged with the next one.
        if (*block).has_next() && (*(*block).next()).free() {
            merge_blocks(block);
            shrink_block(block, size + adjustment);
        }
        // Check if the current block can be merged with the previous one.
        else if (*block).has_prev() && (*(*block).prev()).free() {
            block = (*block).prev();
            adjustment = get_adjustment(block, align);
            merge_blocks(block);
            shrink_block(block, size + adjustment);
        }
        // Check if there's enough free space to split the current block.
        else if (*block).size() as usize > size + adjustment + HEADER_SIZE {
            shrink_block(block, size + adjustment);
        }
    }

    block
}

unsafe impl GlobalAlloc for Locked<Allocator> {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();
        let size = _layout.size();
        let align = _layout.align();
        let adjustment;

        if let Some(mut block) = find_usable_block(&mut allocator, size, align) {
            block = resize_block(block, size, align);
            adjustment = get_adjustment(block, align);

            (*block).set_free(false);

            (block as usize + HEADER_SIZE + adjustment) as *mut u8
        } else {
            null_mut()
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut allocator = self.lock();
        let adjustment = get_adjustment(_ptr as *mut HeapBlock, _layout.align());
        let block = (_ptr as usize - HEADER_SIZE - adjustment) as *mut HeapBlock;

        // use dealloc_node function
        dealloc_node(&mut allocator, block, (*block).size() as usize);
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

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
