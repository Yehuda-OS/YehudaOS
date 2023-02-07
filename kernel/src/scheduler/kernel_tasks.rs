use x86_64::structures::paging::{PageSize, Size4KiB};

use crate::mutex::Mutex;

use super::SchedulerError;

const MAX_STACK_SIZE: u64 = 1024 * 4 * 20; // 80KiB
const STACK_START: u64 = 0x2000_0000;

static STACK_BITMAP: Mutex<u64> = Mutex::new(0);

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

            // There is one page of unused space between the stacks.
            return Some(STACK_START + i * (MAX_STACK_SIZE + Size4KiB::SIZE));
        }
    }

    // Return `None` if no free stack was found.
    None
}

/// Free a kernel's task stack after it is finished.
/// 
/// # Arguments
/// - `stack_pointer` - The stack pointer of the kernel's stack.
/// The function assumes the stack pointer is in the range of the stack the task has received.
fn deallocate_stack(stack_pointer: u64) {
    todo!()
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
