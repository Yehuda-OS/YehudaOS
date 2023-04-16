use super::Process;
use crate::{mutex::Mutex, queue::Queue};

static mut TERMINATE_PROC_QUEUE: Mutex<Queue<Process>> = Mutex::new(Queue::new());

pub unsafe fn add_to_queue(p: Process) {
    TERMINATE_PROC_QUEUE.lock().enqueue(p);
}

pub extern "C" fn terminate_from_queue(_: *mut u64) -> i32 {
    loop {
        for _ in 0..20 {} // wait a little, else the TERMINATE_PROC_QUEUE mutex will corrupt and the system will crash
        unsafe { TERMINATE_PROC_QUEUE.lock() }.dequeue();
    }
}
