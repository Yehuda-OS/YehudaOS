extern crate alloc;
use alloc::vec::Vec;
use core::result::Result::*;

pub struct BlkDev(Vec<u8>); // BlkDev.0 is the file map

impl core::clone::Clone for BlkDev {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl BlkDev {
    pub const DEVICE_SIZE: usize = 1024 * 1024;

    fn new(data: Vec<u8>) -> Result<Self, core::fmt::Error> {
        // Set the initial data of the block device to the provided data
        let mut filemap = data;

        // Ensure that the block device has at least the minimum required size
        if filemap.len() < Self::DEVICE_SIZE {
            filemap.resize(Self::DEVICE_SIZE, 0);
        }

        Ok(BlkDev(filemap))
    }

    pub unsafe fn read(self, addr: isize, size: usize, ans: *mut u8) {
        let src: *const u8 = (self.0.as_ptr().addr() as isize + addr) as usize as *const u8;
        core::ptr::copy_nonoverlapping(src, ans, size);
    }

    pub unsafe fn write(self, addr: isize, size: usize, data: *mut u8) {
        let dst: *mut u8 = (self.0.as_ptr().addr() as isize + addr) as usize as *mut u8;
        core::ptr::copy_nonoverlapping(data, dst, size);
    }
}
