static mut tss_entry: TaskStateSegment = TaskStateSegment {};

#[repr(packed)]
pub struct TaskStateSegment {
    // TODO
}

/// Returns the address of the Task State Segment.
pub fn get_tss_address() -> u64 {
    unsafe { &tss_entry as *const _ as u64 }
}
