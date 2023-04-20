use alloc::collections::LinkedList;

use super::Process;
use crate::mutex::Mutex;

static TERMINATE_PROC_QUEUE: Mutex<LinkedList<Process>> = Mutex::new(LinkedList::new());

pub unsafe fn add_to_queue(p: Process) {
    if let Some(mut q) = TERMINATE_PROC_QUEUE.try_lock() {
        q.push_back(p);
    }
}

pub extern "C" fn terminate_from_queue(_: *mut u64) -> i32 {
    let mut q;

    loop {
        q = TERMINATE_PROC_QUEUE.lock();

        q.pop_front();

        drop(q);

        // Call `sched_yield`.
        unsafe { core::arch::asm!("mov rax, 0x18; syscall") }
    }
}
