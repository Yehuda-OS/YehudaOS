extern crate alloc;
use alloc::vec;
use vec::Vec;

pub const DEVICE_SIZE: usize = 1024 * 1024;

static mut DATA: Vec<u8> = Vec::new();

/// Initialize the block device.
/// Must be called before performing any other operation on the block device.
pub fn init() {
    unsafe { DATA = vec![0; DEVICE_SIZE] }
}

/// Read from the block device.
///
/// # Arguments
/// - `addr` - The offset in the block device to start reading from.
/// - `size` - The amount of bytes to read.
/// - `ans` - The buffer to read into.
///
/// # Safety
/// This operation is unafe because it uses pointers.
pub unsafe fn read(addr: usize, size: usize, ans: *mut u8) {
    core::ptr::copy_nonoverlapping(DATA.as_ptr().add(addr), ans, size);
}

/// Write to the block device.
///
/// # Arguments
/// - `addr` - The offset ein the block device to start writing to.
/// - `size` - The amount of bytes to write.
/// - `data` - The buffer to write from.
///
/// # Safety
/// This operation is unafe because it uses pointers.
pub unsafe fn write(addr: usize, size: usize, data: *const u8) {
    core::ptr::copy_nonoverlapping(data, DATA.as_mut_ptr().add(addr), size)
}
