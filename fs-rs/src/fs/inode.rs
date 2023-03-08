use super::blkdev;
use super::FsError;
use super::BLOCK_SIZE;

pub const DIRECT_POINTERS: usize = 12;
const POINTER_SIZE: usize = core::mem::size_of::<usize>();
pub const MAX_FILE_SIZE: usize =
    DIRECT_POINTERS * BLOCK_SIZE + BLOCK_SIZE / POINTER_SIZE * BLOCK_SIZE;

#[derive(Clone, Copy, Debug)]
pub struct Inode {
    pub id: usize,
    pub directory: bool,
    size: usize,
    pub addresses: [usize; DIRECT_POINTERS],
    pub indirect_pointer: usize,
}

impl Inode {
    pub const fn new() -> Self {
        Inode {
            id: 0,
            directory: false,
            size: 0,
            addresses: [0; DIRECT_POINTERS],
            indirect_pointer: 0,
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    /// Sets the size of an inode to `value`.
    /// Deallocates the unused pointers, it is the responsible of the caller to prevent
    /// any dangling pointers.
    ///
    /// # Returns
    /// Returns a `MaximumSizeExceeded` error if the new size exceeds the maximum file size.
    pub fn set_size(&mut self, value: usize) -> Result<(), FsError> {
        if value > MAX_FILE_SIZE {
            return Err(super::FsError::MaximumSizeExceeded);
        }

        if value / BLOCK_SIZE <= DIRECT_POINTERS && self.indirect_pointer != 0 {
            super::deallocate_block(self.indirect_pointer);
            self.indirect_pointer = 0;
        }

        self.size = value;

        Ok(())
    }

    /// Returns the `index`th pointer of the inode or `MaximumSizeExceeded` if the `index`
    /// exceeds the maximum file size divided by the block size.
    ///
    /// # Arguments
    /// - `index` - The index of the pointer.
    pub fn get_ptr(&self, index: usize) -> Result<usize, FsError> {
        let offset;
        let mut ptr: usize = 0;

        if index < DIRECT_POINTERS {
            return Ok(self.addresses[index]);
        }

        offset = (index - DIRECT_POINTERS) * POINTER_SIZE;
        if offset > BLOCK_SIZE {
            return Err(FsError::MaximumSizeExceeded);
        }
        if self.indirect_pointer == 0 {
            return Ok(0);
        }

        unsafe {
            blkdev::read(
                self.indirect_pointer + offset,
                POINTER_SIZE,
                &mut ptr as *mut _ as *mut u8,
            );
        }

        Ok(ptr)
    }

    /// Set the value of the `index`th pointer.
    ///
    /// # Arguments
    /// - `index` - The index of the pointer.
    /// - `value` - The value to change to.
    ///
    /// # Returns
    /// - `MaximumSizeExceeded` if the pointer exceeds the file's size.
    /// - `NotEnoughDiskSpace` if there is no free space for the pointer.
    /// - `Ok` otherwise.
    pub fn set_ptr(&mut self, index: usize, value: usize) -> Result<(), FsError> {
        let offset;

        if index * BLOCK_SIZE > self.size {
            return Err(FsError::MaximumSizeExceeded);
        }

        if index < DIRECT_POINTERS {
            self.addresses[index] = value;

            return Ok(());
        }

        offset = (index - DIRECT_POINTERS) * POINTER_SIZE;
        if self.indirect_pointer == 0 {
            self.indirect_pointer = super::allocate_block().ok_or(FsError::NotEnoughDiskSpace)?;
            // SAFETY: We checked that the allocation succeeded.
            unsafe { blkdev::set(self.indirect_pointer, BLOCK_SIZE, 0) }
        }
        unsafe {
            blkdev::write(
                self.indirect_pointer + offset,
                POINTER_SIZE,
                &value as *const _ as *const u8,
            );
        };

        Ok(())
    }
}
