use x86_64::{
    structures::paging::{PageSize, PhysFrame, Size4KiB},
    VirtAddr,
};

use crate::memory;
use crate::mutex::Mutex;

use super::SchedulerError;

const MAX_STACK_SIZE: u64 = 1024 * 4 * 20; // 80KiB
const STACK_START: u64 = 0x2000_0000;

static STACK_BITMAP: Mutex<u64> = Mutex::new(0);

fn get_stack_address(index: u64) -> u64 {
    // There is one page of unused space between the stacks.
    STACK_START + index * (MAX_STACK_SIZE + Size4KiB::SIZE)
}

/// Allocates a virtual address for a kernel's task stack.
///
/// # Returns
/// Returns the virtual address for the new stack or `None` if the maximum amount of
/// kernel tasks have been exceeded.
fn allocate_stack() -> Option<u64> {
    let mut bitmap = STACK_BITMAP.lock();

    for i in 0..64 {
        // Check if the stack is unused.
        if *bitmap & (1 << i) == 0 {
            // Set the stack as used.
            *bitmap ^= 1 << i;

            return Some(get_stack_address(i));
        }
    }

    // Return `None` if no free stack was found.
    None
}

/// Free a kernel's task stack after it is finished.
/// Frees all the memory that was mapped to this stack and marks it as free in the bitmap.
/// 
/// # Arguments
/// - `stack_pointer` - The stack pointer of the kernel's stack.
/// The function assumes the stack pointer is in the range of the stack the task has received.
pub fn deallocate_stack(stack_pointer: u64) {
    let index = (stack_pointer + MAX_STACK_SIZE - STACK_START) / (MAX_STACK_SIZE + Size4KiB::SIZE);
    // Get the lower edge of the stack.
    let lower = VirtAddr::new(get_stack_address(index - 1) + Size4KiB::SIZE);
    let higher = VirtAddr::new(get_stack_address(index));

    for addr in (lower..higher).step_by(Size4KiB::SIZE as usize) {
        if let Ok(page) = memory::vmm::virtual_to_physical(memory::get_page_table(), addr) {
            // UNWRAP: The entry is unused because we checked if it is mapped
            // and the page table should not be null.
            memory::vmm::unmap_address(page, addr).unwrap();
            // UNWRAP: The page was returned from the `virtual_to_physical` function.
            unsafe { memory::page_allocator::free(PhysFrame::from_start_address(page).unwrap()) }
        }
    }

    // Clear the stack's bit.
    *STACK_BITMAP.lock() &= !(1 << index);
}

impl super::Process {
    pub fn kernel_task(function: u64) -> Result<Self, SchedulerError> {
        let p = super::Process {
            registers: super::Registers::default(),
            page_table: crate::memory::get_page_table(),
            stack_pointer: allocate_stack().ok_or(SchedulerError::OutOfMemory)?,
            instruction_pointer: function,
            flags: 0,
            kernel_task: true,
        };

        Ok(p)
    }
}
