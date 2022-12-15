extern crate alloc;
use alloc::vec::Vec;
use core::result::{Result, Result::Err, Result::Ok};

pub struct BlkDev(Vec<u8>); // BlkDev.0 is the file map

impl core::clone::Clone for BlkDev {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl BlkDev {
    pub const DEVICE_SIZE: usize = 1024 * 1024;

    pub fn new(data: Vec<u8>) -> Result<Self, &'static str> {
        // Set the initial data of the block device to the provided data
        let mut filemap = data;

        // Ensure that the block device has at least the minimum required size
        if filemap.len() < Self::DEVICE_SIZE {
            filemap.resize(Self::DEVICE_SIZE, 0);
        }

        Ok(BlkDev(filemap))
    }

    pub unsafe fn read(&self, addr: usize, size: usize, ans: *mut u8) {
        for i in 0..size {
            *(ans.add(i)) = self.0[addr + i];
        }
    }

    pub unsafe fn write(&mut self, addr: usize, size: usize, data: *mut u8) {
        for i in 0..size {
            self.0[addr + i] = *(data.add(i));
        }
    }
}
