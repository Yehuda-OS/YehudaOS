use super::memory;
use crate::memory::allocator::{Allocator, Locked};
use crate::mutex::Mutex;
use crate::{io, syscalls};
use alloc::collections::{BTreeMap, LinkedList};
use alloc::string::String;
use core::arch::asm;
use core::fmt;
use fs_rs::fs;
use x86_64::{
    structures::paging::{PageSize, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

mod kernel_tasks;
mod loader;
pub mod terminator;

pub const MAX_STACK_SIZE: u64 = 1024 * 20; // 20KiB
const KERNEL_CODE_SEGMENT: u16 = super::gdt::KERNEL_CODE;
const KERNEL_DATA_SEGMENT: u16 = super::gdt::KERNEL_DATA;
const USER_CODE_SEGMENT: u16 = super::gdt::USER_CODE | 3;
const USER_DATA_SEGMENT: u16 = super::gdt::USER_DATA | 3;
const INTERRUPT_FLAG_ON: u64 = 0x200;
const HIGH_PRIORITY_RELOAD: u8 = 2;

static mut CURR_PROC: Option<Process> = None;
static mut LOW_PRIORITY: LinkedList<Process> = LinkedList::new();
static mut HIGH_PRIORITY: LinkedList<Process> = LinkedList::new();
static mut HIGH_PRIORITY_VALUE: u8 = HIGH_PRIORITY_RELOAD;
static mut WAITING_QUEUE: BTreeMap<i64, (Process, *mut i32)> = BTreeMap::new();

static mut TSS_ENTRY: TaskStateSegment = TaskStateSegment {
    reserved0: 0,
    rsp0: 0,
    rsp1: 0,
    rsp2: 0,
    reserved1: 0,
    ist1: 0,
    ist2: 0,
    ist3: 0,
    ist4: 0,
    ist5: 0,
    ist6: 0,
    ist7: 0,
    reserved2: 0,
    reserved3: 0,
    io_permission_bitmap: 0,
};

#[derive(Debug)]
pub enum SchedulerError {
    OutOfMemory,
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SchedulerError::OutOfMemory => write!(f, "not enough memory to create a process"),
        }
    }
}

#[repr(packed)]
#[allow(unused)]
pub struct TaskStateSegment {
    reserved0: u32,
    rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    reserved1: u64,
    ist1: u64,
    ist2: u64,
    ist3: u64,
    ist4: u64,
    ist5: u64,
    ist6: u64,
    ist7: u64,
    reserved2: u64,
    reserved3: u16,
    io_permission_bitmap: u16,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Registers {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
}

#[repr(C)]
pub struct Process {
    pub registers: Registers,
    pub stack_pointer: u64,
    pub page_table: PhysAddr,
    pub instruction_pointer: u64,
    pub flags: u64,
    pid: i64,
    stack_start: VirtAddr,
    cwd_path: String,
    cwd: usize,
    kernel_task: bool,
    allocator: Locked<Allocator>,
}

impl Drop for Process {
    fn drop(&mut self) {
        if self.kernel_task {
            kernel_tasks::deallocate_stack(self.stack_pointer);
        } else {
            memory::vmm::page_table_walker(self.page_table, &|virt, physical| {
                if virt.as_u64() < memory::HHDM_OFFSET {
                    memory::vmm::unmap_address(self.page_table, virt).unwrap();
                    unsafe {
                        memory::page_allocator::free(PhysFrame::from_start_address_unchecked(
                            physical,
                        ))
                    }
                }
            });
            // SAFETY: The page table has been created with `create_page_table`.
            unsafe {
                memory::page_allocator::free(PhysFrame::from_start_address_unchecked(
                    self.page_table,
                ))
            }
        }
    }
}

impl Process {
    pub const fn cwd(&self) -> usize {
        self.cwd
    }

    pub fn cwd_path(&self) -> &str {
        &self.cwd_path
    }

    /// Set the current working directory of the process to `value`.
    ///
    /// # Panics
    /// If `value` does not exist in the filesystem.
    pub fn set_cwd(&mut self, value: &str) {
        self.cwd_path = String::from(value);
        self.cwd = fs::get_file_id(value, None).unwrap();
    }

    pub const fn kernel_task(&self) -> bool {
        self.kernel_task
    }

    pub const fn stack_start(&self) -> VirtAddr {
        self.stack_start
    }

    pub const fn pid(&self) -> i64 {
        self.pid
    }

    pub const fn allocator(&self) -> &Locked<Allocator> {
        &self.allocator
    }
}

/// Returns a new process ID.
/// Assumes that no more than 2 ^ 63 processes will ever be created.
fn allocate_pid() -> i64 {
    static PID_COUNTER: Mutex<i64> = Mutex::new(0);
    let mut counter = PID_COUNTER.lock();
    let pid = *counter;

    *counter += 1;

    pid
}

/// Get the `rsp0` field from the TSS.
pub fn get_kernel_stack() -> u64 {
    unsafe { TSS_ENTRY.rsp0 }
}

/// Returns a mutable reference to the currently running process.
///
/// # Safety
/// Should not be used in a multi-threaded situation.
pub unsafe fn get_running_process() -> &'static mut Option<Process> {
    &mut CURR_PROC
}

/// Searches for a process in the different queues.
///
/// # Arguments
/// - `pid` - The process ID of the process to search.
///
/// # Returns
/// `true` if the process was found and `false` if it wasn't.
///
/// # Safety
/// Should not be used in a multi-threaded situation.
pub unsafe fn search_process(pid: i64) -> bool {
    let queues = [&mut LOW_PRIORITY, &mut HIGH_PRIORITY];

    for queue in queues {
        for element in queue {
            if element.pid() == pid {
                return true;
            }
        }
    }
    for element in WAITING_QUEUE.values() {
        if element.0.pid() == pid {
            return true;
        }
    }

    false
}

/// Add a process to the waiting processes.
/// The waiting processes are processes who are waiting for a child process to terminate.
/// A process will not continue its execution as long as it is in the waiting processes.
///
/// # Arguments
/// - `pid` - The process ID of the process to wait for.
/// The function assumes the process exist.
/// - `parent` - The process who's waiting.
/// - `wstatus` - A buffer for the future child process' exit code.
///
/// # Safety
/// - `wstatus` must be valid for writes.
/// - Should not be used in a multi-threaded situation.
pub unsafe fn wait_for(pid: i64, parent: Process, wstatus: *mut i32) {
    WAITING_QUEUE.insert(pid, (parent, wstatus));
}

/// Notify a waiting parent of the termination of its child, if it exists.
///
/// # Arguments
/// - `p` - The child process that has finished.
/// - `status` - The exit code of the child process.
///
/// # Safety
/// Should not be used in a multi-threaded situation.
pub unsafe fn stop_waiting_for(p: &Process, status: i32) {
    if let Some(parent) = WAITING_QUEUE.remove(&p.pid()) {
        add_to_the_queue(parent.0);
        *parent.1 = status;
    }
}

/// function that push process into the process queue
///
/// # Arguments
/// - `p` - the process
pub fn add_to_the_queue(p: Process) {
    // SAFETY: The shceduler should not be referenced in a multithreaded situation.
    unsafe {
        if p.kernel_task {
            HIGH_PRIORITY.push_back(p);
        } else {
            LOW_PRIORITY.push_back(p);
        }
    }
}

/// Re-add the current process to the process queue and set the current process to `None`.
///
/// # Panics
/// Panics if there is no current process.
pub fn switch_current_process() {
    // SAFETY: `CURR_PROC` is a valid `Process` struct and the ownership of it is now moved to the
    // process queue, so no resources are leaked.
    unsafe {
        let curr = core::ptr::read(CURR_PROC.as_ref().unwrap());

        core::ptr::write(&mut CURR_PROC, None);
        add_to_the_queue(curr);
    }
}

/// Load a process from the queue.
///
/// # Panics
/// Panics if the process queue is empty.
pub unsafe fn load_from_queue() -> ! {
    // Take high priority processes if the amount of high priority processes that were ran since
    // the last low priority process is less than the reload value or if there are no low
    // priority processes waiting.
    let p = if (HIGH_PRIORITY_VALUE > 0 && !HIGH_PRIORITY.is_empty()) || LOW_PRIORITY.is_empty() {
        if HIGH_PRIORITY_VALUE > 0 {
            HIGH_PRIORITY_VALUE -= 1;
        }

        HIGH_PRIORITY
            .pop_front()
            .expect("No processes in the queue")
    } else {
        HIGH_PRIORITY_VALUE = HIGH_PRIORITY_RELOAD;

        LOW_PRIORITY.pop_front().expect("No processes in the queue")
    };

    if let Some(process) = &CURR_PROC {
        add_to_the_queue(core::ptr::read(process))
    }
    core::ptr::write(&mut CURR_PROC, Some(p));
    load_context(CURR_PROC.as_ref().unwrap());
}

/// Returns the address of the Task State Segment.
pub fn get_tss_address() -> u64 {
    unsafe { &TSS_ENTRY as *const _ as u64 }
}

/// Load kernel's stack pointer to the TSS and load the
/// TSS segment selector to the task register.
///
/// # Safety
/// This function is unsafe because it requires a valid GDT with a TSS segment descriptor.
pub unsafe fn load_tss() {
    asm!("mov {0}, rsp", out(reg)TSS_ENTRY.rsp0);
    asm!("mov {0}, rsp", out(reg)TSS_ENTRY.ist1);
    asm!("ltr ax", in("ax")super::gdt::TSS);
}

/// Start running a user process in ring 3.
///
/// # Arguments
/// - `p` - The process' data structure.
///
/// # Safety
/// This function is unsafe because it jumps to a code at a specific
/// address and deletes the entire call stack.
pub unsafe fn load_context(p: &Process) -> ! {
    let (code_segment, data_segment) = if p.kernel_task {
        (KERNEL_CODE_SEGMENT, KERNEL_DATA_SEGMENT)
    } else {
        (USER_CODE_SEGMENT, USER_DATA_SEGMENT)
    };
    let p_address = p as *const Process as u64;

    memory::load_tables_to_cr3(p.page_table);
    // Write the address of the process to later use it in the syscall handler.
    asm!("swapgs");
    io::wrmsr(syscalls::KERNEL_GS_BASE, p_address);
    asm!("swapgs");
    // Move the user data segment selector to the segment registers and push
    // the future `ss`, `rsp`, `rflags`, `cs` and `rip` that will later be popped by `iretq`.
    asm!("
    mov ds, {0:x}
    mov es, {0:x}
    mov fs, {0:x}

    push {0:r}
    push {rsp}
    push {flags}
    push {1:r}
    push {rip}
    ",
        in(reg)data_segment, in(reg)code_segment,
        flags=in(reg)p.flags,
        rsp=in(reg)p.stack_pointer, rip=in(reg)p.instruction_pointer
    );
    // Push the future `rbx` and `rbp` to later pop them.
    asm!("
    push {rbx}
    push {rbp}
    ",
            rbx=in(reg)p.registers.rbx,
            rbp=in(reg)p.registers.rbp,
    );
    // Pop `rbx` and `rbp` that we pushed earlier and perform the return
    // after loading the general purpose register with the appropriate values.
    asm!("
    pop rbp
    pop rbx
    iretq",
        in("rax")p.registers.rax,
        in("rcx")p.registers.rcx,
        in("rdx")p.registers.rdx,
        in("rsi")p.registers.rsi,
        in("rdi")p.registers.rdi,
        in("r8")p.registers.r8,
        in("r9")p.registers.r9,
        in("r10")p.registers.r10,
        in("r11")p.registers.r11,
        in("r12")p.registers.r12,
        in("r13")p.registers.r13,
        in("r14")p.registers.r14,
        in("r15")p.registers.r15,
        options(noreturn)
    );
}

/// Create a page table for a process and copy the higher half of the kernel's page table to it
/// because the kernel's memory is at the higher half of the address space.
///
/// # Returns
/// The address of the new page table or `None` if there is no free space for a page table.
///
/// # Safety
/// A valid kernel's page table is required.
unsafe fn create_page_table() -> Option<PhysAddr> {
    let table = memory::vmm::create_page_table()?;
    let high_kernel_table = memory::get_page_table() + Size4KiB::SIZE / 2;
    let high_user_table = table + Size4KiB::SIZE / 2;

    core::ptr::copy_nonoverlapping(
        (high_kernel_table.as_u64() + memory::HHDM_OFFSET) as *const u8,
        (high_user_table.as_u64() + memory::HHDM_OFFSET) as *mut u8,
        Size4KiB::SIZE as usize / 2,
    );

    Some(table)
}
