extern crate alloc;
use alloc::vec;
use vec::Vec;

pub struct BlkDev {
    data: Vec<u8>,
}

impl core::clone::Clone for BlkDev {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}

impl BlkDev {
    pub const DEVICE_SIZE: usize = 1024 * 1024;

    pub fn new() -> Self {
        Self {
            data: vec![0; BlkDev::DEVICE_SIZE],
        }
    }

    pub unsafe fn read(&self, addr: usize, size: usize, ans: *mut u8) {
        core::ptr::copy_nonoverlapping(self.data.as_ptr().add(addr), ans, size);
    }

    pub unsafe fn write(&mut self, addr: usize, size: usize, data: *const u8) {
        core::ptr::copy_nonoverlapping(data, self.data.as_mut_ptr().add(addr), size)
    }
}
