use crate::mutex::MutexGuard;
use crate::{memory, mutex::Mutex};
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};
use heap_block::HeapBlock;
use x86_64::{
    structures::paging::{PageSize, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

mod heap_block;

const KERNEL_HEAP_START: u64 = 0xffff_faaa_0000_0000;
pub const USER_HEAP_START: u64 = 0x4444_4444_0000;
pub const DEFAULT_ALIGNMENT: usize = 16;

const HEADER_SIZE: u64 = core::mem::size_of::<HeapBlock>() as u64;

#[global_allocator]
pub static mut ALLOCATOR: Locked<Allocator> =
    Locked::<Allocator>::new(Allocator::new(KERNEL_HEAP_START, PhysAddr::zero()));

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

    pub fn set_page_table(&mut self, page_table: PhysAddr) {
        self.page_table = page_table;
    }
}

/// Returns the required adjustment of a data block to match the required allocation alignment.
///
/// # Arguments
/// - `addr` - Pointer to the heap block.
/// - `align` - The required alignment.
fn get_adjustment(addr: *mut HeapBlock, align: u64) -> u64 {
    let data_start_address = unsafe { addr.add(1) } as u64;

    align - data_start_address % align
}

/// Request pages from the page allocator until there is enough space for the required data size
/// and create a `HeapBlock` instance at the start of the allocated space.
///
/// # Arguments
/// - `allocator` - The `Allocator` instance that is being used.
/// - `last` - The last heap block.
/// - `size` - The required allocation size.
/// - `align` - The required alignment for the allocation's start address.
///
/// # Returns
/// A pointer to the created `HeapBlock`, or [`None`] if the allocation failed.
fn alloc_node(
    allocator: &mut Allocator,
    last: *mut HeapBlock,
    size: u64,
    align: u64,
) -> Option<*mut HeapBlock> {
    let start = VirtAddr::new(allocator.heap_start + allocator.pages * Size4KiB::SIZE);
    let mut current_size = 0;
    let adjustment = get_adjustment(start.as_mut_ptr(), align);
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    let allocated;
    let required_pages = if (size + adjustment) % Size4KiB::SIZE == 0 {
        (size + adjustment) / Size4KiB::SIZE
    } else {
        (size + adjustment) / Size4KiB::SIZE + 1
    };
    let mut success = true;

    for _ in 0..required_pages {
        if let Some(page) = super::page_allocator::allocate() {
            allocator.pages += 1;
            if super::vmm::map_address(allocator.page_table, start + current_size, page, flags)
                .is_err()
            {
                success = false;

                break;
            }
            current_size += Size4KiB::SIZE;
        } else {
            success = false;

            break;
        }
    }
    if !success {
        // If the allocation fails, unmap everything we mapped so far.
        while current_size > 0 {
            allocator.pages -= 1;
            // SAFETY: The page is valid because we allocated it with `allocate`.
            unsafe {
                // UNWRAP: The entry is not unused because we just mapped it
                // and if the page table is null the call to `map_address` would
                // return `None` and this code would never run.
                super::page_allocator::free(
                    PhysFrame::from_start_address(
                        super::vmm::virtual_to_physical(allocator.page_table, start + current_size)
                            .unwrap(),
                    )
                    // UNWRAP: The page is aligned.
                    .unwrap(),
                );
            }
            // UNWRAP: Same as above.
            super::vmm::unmap_address(allocator.page_table, start + current_size).unwrap();
            current_size -= Size4KiB::SIZE;
        }

        return None;
    }
    memory::flush_tlb_cache();
    // Allocation succeeded, add the allocated block to the list.
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
unsafe fn dealloc_node(allocator: &mut Allocator, mut block: *mut HeapBlock) {
    (*block).set_free(true);
    if (*block).has_next() && (*(*block).next()).free() {
        merge_blocks(block);
    }
    if (*block).has_prev() && (*(*block).prev()).free() {
        block = (*block).prev();
        merge_blocks(block);
    }

    if !(*block).has_next() {
        while (*block).size() > Size4KiB::SIZE {
            super::page_allocator::free(
                PhysFrame::from_start_address(
                    super::vmm::virtual_to_physical(
                        allocator.page_table,
                        VirtAddr::new(
                            allocator.heap_start + Size4KiB::SIZE * (allocator.pages - 1),
                        ),
                    )
                    // UNWRAP: If the page table is null any allocation would fail and
                    // the entry is used because we keep track of what we mapped.
                    .unwrap(),
                )
                // UNWRAP: The address is aligned because `heap_start` is aligned.
                .unwrap(),
            );
            super::vmm::unmap_address(
                allocator.page_table,
                VirtAddr::new(allocator.heap_start + Size4KiB::SIZE * (allocator.pages - 1)),
            )
            // UNWRAP: If the page table is null any allocation would fail and
            // the entry is used because we keep track of what we mapped.
            .unwrap();

            (*block).set_size((*block).size() - Size4KiB::SIZE);
            allocator.pages -= 1;
        }

        if (*block).size() == 0 {
            (*(*block).prev()).set_has_next(false);
            (*(*block).prev()).set_size((*(*block).prev()).size() + HEADER_SIZE as u64);
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
    size: u64,
    align: u64,
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
            return alloc_node(allocator, curr, size, align);
        } else if (*curr).free() && (*curr).size() >= size + curr_adjustment {
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

    (*block).set_size((*block).size() + next.size() + HEADER_SIZE);
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
unsafe fn shrink_block(block: *mut HeapBlock, size: u64) {
    let has_next = (*block).has_next();
    let extra = (*block).size() - size;

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
unsafe fn resize_block(mut block: *mut HeapBlock, size: u64, align: u64) -> *mut HeapBlock {
    let mut adjustment = get_adjustment(block, align);

    if (*block).size() > size + adjustment {
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
        else if (*block).size() > size + adjustment + HEADER_SIZE {
            shrink_block(block, size + adjustment);
        }
    }

    block
}

/// Used for debugging.
#[allow(unused)]
unsafe fn print_list(first: *mut HeapBlock) {
    use crate::println;
    let mut curr = first;

    println!("\n\n|LIST|");
    while curr != null_mut() {
        println!("{:p} : {:?}, size: {:#x}", curr, *curr, (*curr).size());
        curr = (*curr).next();
    }
}

impl Locked<Allocator> {
    pub unsafe fn global_alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc(layout)
    }

    pub unsafe fn global_dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.dealloc(ptr, layout);
    }

    pub unsafe fn global_realloc(&self, ptr: *mut u8, new_size: usize) -> *mut u8 {
        self.realloc(ptr, Layout::from_size_align(0, 1).unwrap(), new_size)
    }

    pub fn get_page_table(&self) -> PhysAddr {
        self.inner.lock().page_table
    }
}

unsafe impl GlobalAlloc for Locked<Allocator> {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();
        let size = _layout.size() as u64;
        let align = _layout.align() as u64;
        let adjustment;

        if let Some(mut block) = find_usable_block(&mut allocator, size, align) {
            block = resize_block(block, size, align);
            adjustment = get_adjustment(block, align);
            // Zero out all the unused bytes.
            for i in (block as u64 + HEADER_SIZE)..(block as u64 + HEADER_SIZE + adjustment) {
                *(i as *mut u8) = 0;
            }

            (*block).set_free(false);

            (block as u64 + HEADER_SIZE + adjustment) as *mut u8
        } else {
            null_mut()
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut allocator;
        let block;

        if _ptr.is_null() {
            return;
        }

        allocator = self.lock();
        block = HeapBlock::get_ptr_block(_ptr);
        dealloc_node(&mut allocator, block);
    }
}

/// A wrapper around crate::mutex::Mutex to permit trait implementations.
pub struct Locked<A> {
    inner: Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> MutexGuard<A> {
        self.inner.lock()
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
