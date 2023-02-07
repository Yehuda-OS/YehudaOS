use crate::mutex::Mutex;

use super::SchedulerError;

const MAX_STACK_SIZE: u64 = 1024 * 4 * 20; // 80KiB
const STACK_START: u64 = 0x1000_0000;

static STACK_BITMAP: Mutex<u64> = Mutex::new(0);

fn allocate_stack() -> Option<u64> {
    let mut bitmap = STACK_BITMAP.lock();

    if *bitmap != !0 {
        todo!()
    } else {
        None
    }
}

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
