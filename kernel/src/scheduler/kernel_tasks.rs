use super::MAX_STACK_SIZE;
use alloc::string::String;
use x86_64::{
    structures::paging::{PageSize, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::memory::{self, allocator};
use crate::mutex::Mutex;

use super::SchedulerError;

const STACK_START: u64 = 0x4000_0000;

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

/// Calls the exit syscall.
/// This function will be automatically pushed to every kernel's task return address.
unsafe fn terminate_task() {
    core::arch::asm!(
        "
        mov rdi, 0
        mov edi, eax
        mov rax, 0x3c
        syscall
    "
    );
}

/// Free a kernel's task stack after it is finished.
/// Frees all the memory that was mapped to this stack and marks it as free in the bitmap.
///
/// # Arguments
/// - `stack_pointer` - The stack pointer of the kernel's stack.
/// The function assumes the stack pointer is in the range of the stack the task has received.
pub fn deallocate_stack(stack_pointer: u64) {
    let index = (stack_pointer + MAX_STACK_SIZE - STACK_START) / (MAX_STACK_SIZE + Size4KiB::SIZE);
    let higher = VirtAddr::new(get_stack_address(index));
    // Get the lower edge of the stack.
    let lower = higher - MAX_STACK_SIZE;

    for addr in (lower..higher).step_by(Size4KiB::SIZE as usize) {
        if let Ok(page) = memory::vmm::virtual_to_physical(memory::get_page_table(), addr) {
            // UNWRAP: The entry is unused because we checked if it is mapped
            // and the page table should not be null.
            memory::vmm::unmap_address(memory::get_page_table(), addr).unwrap();
            // UNWRAP: The page was returned from the `virtual_to_physical` function.
            unsafe { memory::page_allocator::free(PhysFrame::from_start_address(page).unwrap()) }
        }
    }

    // Clear the stack's bit.
    *STACK_BITMAP.lock() &= !(1 << index);
}

impl super::Process {
    /// Create a new kernel task.
    ///
    /// # Arguments
    /// - `function` - The function that will be ran.
    /// - `param` - The parameter that will be sent to the function.
    ///
    /// # Returns
    /// A `Process` struct for the task on success or an `OutOfMemory` error on fail.
    pub fn new_kernel_task<T>(
        function: extern "C" fn(*mut T) -> i32,
        param: *mut T,
    ) -> Result<Self, SchedulerError> {
        const POINTER_SIZE: u64 = 8;
        let stack_page = memory::page_allocator::allocate().ok_or(SchedulerError::OutOfMemory)?;
        // UNWRAP: Assume the maximum amount of threads is not exceeded.
        let stack = allocate_stack().unwrap();
        let mut p = super::Process {
            registers: super::Registers::default(),
            page_table: memory::get_page_table(),
            stack_pointer: stack,
            instruction_pointer: function as u64,
            flags: super::INTERRUPT_FLAG_ON,
            pid: -1,
            kernel_task: true,
            stack_start: VirtAddr::new(stack),
            cwd_path: String::from("/"),
            cwd: 0,
            allocator: allocator::Locked::new(allocator::Allocator::new(0, PhysAddr::zero())),
        };

        memory::vmm::map_address(
            p.page_table,
            VirtAddr::new(p.stack_pointer - Size4KiB::SIZE),
            stack_page,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        )
        .map_err(|_| SchedulerError::OutOfMemory)?;
        p.registers.rdi = param as u64;
        // Push the return address to the task's stack.
        unsafe {
            *((stack_page.start_address().as_u64() + Size4KiB::SIZE - POINTER_SIZE
                + memory::HHDM_OFFSET) as *mut u64) = terminate_task as u64
        }
        p.stack_pointer -= POINTER_SIZE;

        Ok(p)
    }
}
