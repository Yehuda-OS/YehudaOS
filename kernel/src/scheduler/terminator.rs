use super::Process;
use crate::mutex::Mutex;
use alloc::vec::Vec;

static mut TERMINATE_PROC_QUEUE: Mutex<Vec<Process>> = Mutex::new(Vec::new());

pub unsafe fn add_to_queue(p: Process) {
    TERMINATE_PROC_QUEUE.lock().push(p);
}

pub extern "C" fn terminate_from_queue(_: *mut u64) -> ! {
    loop {
        if unsafe { TERMINATE_PROC_QUEUE.lock() }.is_empty() {
            continue;
        } else {
            unsafe { TERMINATE_PROC_QUEUE.lock() }.pop();
        }
    }
}
