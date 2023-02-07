use crate::mutex::Mutex;

const MAX_STACK_SIZE: u64 = 1024 * 4 * 20; // 80KiB
const STACK_START: u64 = 0x1000_0000;

static STACK_BITMAP: Mutex<u64> = Mutex::new(0);

fn allocate_stack() {}

fn deallocate_stack(stack_pointer: u64) {}

impl super::Process {
    pub fn kernel_task(function: u64) -> Self {
        let p = super::Process {
            registers: super::Registers::default(),
            page_table: todo!(),
            stack_pointer: todo!(),
            instruction_pointer: todo!(),
            flags: todo!(),
            kernel_task: todo!(),
        };
    }
}
