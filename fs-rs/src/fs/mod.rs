pub mod blkdev;
mod inode;

extern crate alloc;

use alloc::boxed::Box;
use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};
use blkdev::BlkDev;
use core::ops::DerefMut;
use core::option::Option::None;
use core::result::{Result, Result::Err, Result::Ok};
use core::slice;
use inode::Inode;

pub type DirList = Vec<DirListEntry>;

const FS_MAGIC: [u8; 4] = *b"FSRS";
const CURR_VERSION: u8 = 0x1;
const DIRECT_POINTERS: usize = 12;
const FILE_NAME_LEN: usize = 11;
const BLOCK_SIZE: usize = 16;
const BITS_IN_BYTE: usize = 8;
const BYTES_PER_INODE: usize = 16 * 1024;
const POINTER_SIZE: usize = core::mem::size_of::<usize>();
const MAX_FILE_SIZE: usize = DIRECT_POINTERS * BLOCK_SIZE + BLOCK_SIZE / POINTER_SIZE * BLOCK_SIZE;

#[derive(Debug)]
pub enum WriteError {
    NotEnoughDiskSpace,
    MaximumSizeExceeded,
    FileNotFound,
}

#[derive(Debug)]
pub enum SetLenError {
    MaximumSizeExceeded,
    FileNotFound,
}

pub struct Fs {
    blkdev: BlkDev,
    disk_parts: DiskParts,
}

impl core::clone::Clone for Fs {
    fn clone(&self) -> Self {
        Self {
            blkdev: self.blkdev.clone(),
            disk_parts: self.disk_parts,
        }
    }
}

struct Header {
    magic: [u8; 4],
    version: u8,
}

struct DiskParts {
    block_bit_map: usize,
    inode_bit_map: usize,
    root: usize,
    unused: usize,
    data: usize,
}

impl core::clone::Clone for DiskParts {
    fn clone(&self) -> Self {
        Self {
            block_bit_map: self.block_bit_map.clone(),
            inode_bit_map: self.inode_bit_map.clone(),
            root: self.root.clone(),
            unused: self.unused.clone(),
            data: self.data.clone(),
        }
    }
}

impl Copy for DiskParts {}

#[derive(Clone)]
pub struct DirListEntry {
    pub name: &'static str,
    pub is_dir: bool,
    pub file_size: usize,
}

#[derive(Clone)]
struct DirEntry {
    name: &'static str,
    id: usize,
}

/// private functions
impl Fs {
    /// function that returns the root dir (/)
    ///
    /// # Returns
    /// the Inode of the root dir
    fn get_root_dir(&self) -> Inode {
        let mut ans = Inode::new();

        unsafe {
            self.blkdev.read(
                self.disk_parts.root,
                core::mem::size_of::<Inode>(),
                &mut ans as *mut Inode as *mut u8,
            )
        };

        ans
    }

    /// Returns the `Inode` of a file, or `None` if no file was found.
    ///
    /// # Arguments
    /// - `path` - The path to the file.
    /// - `cwd` - The current working directory, used for relative paths.
    fn get_inode(&self, mut path: &str, cwd: Option<Inode>) -> Option<Inode> {
        let mut next_delimiter = path.find('/');
        let mut next_folder;
        let mut inode = self.get_root_dir();
        let mut index: usize = 0;

        if path == "/" {
            return Some(inode);
        }
        // Check if the path is relative
        if path.chars().nth(0).unwrap_or(' ') != '/' {
            if let Some(cwd) = cwd {
                inode = cwd;
            } else {
                return None;
            }
        }

        while next_delimiter != None {
            let mut data: Vec<u8> = vec![0; inode.size];
            unsafe { self.read(inode.id, data.as_mut_slice(), index) };
            let dir_content = unsafe {
                Box::from(slice::from_raw_parts(
                    data.as_ptr() as *const DirEntry,
                    data.len() / core::mem::size_of::<DirEntry>(),
                ))
            };
            path = &path[(next_delimiter.unwrap() + 1)..];
            next_delimiter = path.find('/');
            next_folder = match next_delimiter {
                Some(delimeter) => &path[0..delimeter],
                None => &path[0..],
            };
            while index != dir_content.len() && dir_content[index].name != next_folder {
                index += 1;
            }
            if index >= dir_content.len() {
                return None;
            }
            match self.read_inode(dir_content[index].id) {
                Some(file) => inode = file,
                None => return None,
            }

            index = 0;
        }

        Some(inode)
    }

    /// find the Inode address by id
    ///
    /// # Arguments
    /// - `id` - the Inode's id
    ///
    /// # Returns
    /// the address if the Inode
    fn get_inode_address(&self, id: usize) -> usize {
        self.disk_parts.root + id * core::mem::size_of::<Inode>()
    }

    #[deprecated]
    fn read_file(&self, inode: &Inode) -> Box<&[u8]> {
        let last_pointer: usize = inode.size / BLOCK_SIZE;
        let mut pointer = 0;
        let mut to_read;
        let mut bytes_read = 0;
        let mut buffer = Box::new(vec![0; inode.size]);

        while bytes_read != inode.size {
            to_read = if pointer == last_pointer {
                inode.size % BLOCK_SIZE
            } else {
                BLOCK_SIZE
            };
            unsafe {
                self.blkdev.read(
                    inode.addresses[pointer],
                    to_read,
                    (*buffer).as_mut_ptr().add(bytes_read),
                );
            };
            bytes_read += to_read;
            pointer += 1;
        }

        unsafe { Box::from(slice::from_raw_parts(buffer.as_ptr(), bytes_read)) }
    }

    #[deprecated]
    fn read_dir(&self, inode: &Inode) -> Box<&[DirEntry]> {
        let data = self.read_file(inode);

        unsafe {
            Box::from(slice::from_raw_parts(
                data.as_ptr() as *const DirEntry,
                data.len() / core::mem::size_of::<DirEntry>(),
            ))
        }
    }

    /// Returns `true` if a bit in a bitmap is set to 1.
    ///
    /// # Arguments
    /// - `bitmap_start` - The start of the bitmap.
    /// - `i` - The index in the bitmap.
    fn is_allocated(&self, bitmap_start: usize, i: usize) -> bool {
        let byte_address = bitmap_start + i / BITS_IN_BYTE;
        let offset = i % BITS_IN_BYTE;
        let mut byte: u8 = 0;

        unsafe { self.blkdev.read(byte_address, 1, &mut byte as *mut u8) }

        byte & (1 << offset) != 0
    }

    /// Returns the `Inode` object with a specific ID, or None if the inode is not
    /// associated with any file.
    ///
    /// # Arguments
    /// - `id` - The inode's ID.
    fn read_inode(&self, id: usize) -> Option<Inode> {
        let mut inode = Inode::new();

        if self.is_allocated(self.disk_parts.inode_bit_map, id) {
            unsafe {
                self.blkdev.read(
                    self.get_inode_address(id),
                    core::mem::size_of::<Inode>(),
                    &mut inode as *mut _ as *mut u8,
                )
            }

            Some(inode)
        } else {
            None
        }
    }

    /// a function that writes Inode to the memory
    ///
    /// # Arguments
    /// - `inode` - the Inode that has to be written to the memory
    fn write_inode(&mut self, inode: &Inode) {
        unsafe {
            self.blkdev.write(
                self.get_inode_address(inode.id),
                core::mem::size_of::<Inode>(),
                inode as *const _ as *mut u8,
            )
        };
    }

    /// allocate Inode
    ///
    /// # Returns
    /// the address of the inode if it was allocated or None if no free space was found
    fn allocate_inode(&mut self) -> Option<usize> {
        Some(self.allocate(self.disk_parts.inode_bit_map)?)
    }

    /// allocate a block or Inode
    ///
    /// # Arguments
    /// - `bitmap_start` - the start of the bitmap
    ///
    /// # Returns
    /// the address of the allocated block or Inode
    fn allocate(&mut self, bitmap_start: usize) -> Option<usize> {
        const BITS_IN_BUFFER: usize = 64;
        const BYTES_IN_BUFFER: usize = BITS_IN_BUFFER / BITS_IN_BYTE;
        const ALL_OCCUPIED: usize = 0xFFFFFFFFFFFFFFFF;
        let mut buffer: usize = 0;
        let mut address: usize = bitmap_start;
        // read the bitmap until an unoccupied memory is found
        loop {
            unsafe {
                self.blkdev
                    .read(address, BYTES_IN_BUFFER, &mut buffer as *mut _ as *mut u8)
            };
            address += BYTES_IN_BUFFER;
            if buffer != ALL_OCCUPIED {
                break;
            }

            if address >= self.disk_parts.data {
                return None;
            }
        }
        address -= BYTES_IN_BUFFER;

        // read the buffer until an unoccupied memory is found
        for i in 0..BITS_IN_BUFFER {
            if buffer & (1 << i) == 0 {
                // if the (i)'s bit is 0
                buffer ^= 1 << i; // flip the bit to mark as occupied
                unsafe {
                    self.blkdev.write(
                        address,
                        BYTES_IN_BUFFER,
                        core::mem::transmute(&buffer as *const usize as *mut u8),
                    );
                }
                // get the index in the bitmap
                address -= bitmap_start;
                address += i;

                // once we found unoccupied space, we finished our task
                break;
            }
        }

        Some(address)
    }

    /// deallocate a block or inode by his index in the bitmap
    ///
    /// # Arguments
    /// - `bitmap_start` - the start og the bitmap
    /// - `n` - the index of the block
    fn deallocate(&mut self, bitmap_start: usize, n: usize) {
        let byte_address: usize = bitmap_start + n / BITS_IN_BYTE;
        let mut byte: u8 = 0;
        let offset = n % BITS_IN_BYTE;

        unsafe { self.blkdev.read(byte_address, 1, &mut byte as *mut u8) };
        byte ^= 1 << offset; // flip the bit to mark as unoccupied
        unsafe { self.blkdev.write(byte_address, 1, &mut byte as *mut u8) };
    }

    #[deprecated]
    fn reallocate_blocks(&mut self, inode: &Inode, new_size: usize) -> Result<Inode, &'static str> {
        let required_blocks = new_size / BLOCK_SIZE + (new_size % BLOCK_SIZE != 0) as usize;
        let blocks_to_allocate;
        let mut used_blocks = 0;
        let mut new_addresses = *inode;

        if required_blocks > DIRECT_POINTERS {
            return Err("too many blocks are required");
        }

        while new_addresses.addresses[used_blocks as usize] != 0 {
            used_blocks += 1;
        }

        blocks_to_allocate = (required_blocks as isize - used_blocks) as isize;

        if blocks_to_allocate > 0 {
            for _ in 0..blocks_to_allocate {
                new_addresses.addresses[used_blocks as usize] = self.allocate_block().unwrap();
                used_blocks += 1;
            }
        } else if blocks_to_allocate < 0 {
            for _ in 0..(-blocks_to_allocate) {
                used_blocks -= 1;
                self.deallocate_block(new_addresses.addresses[used_blocks as usize]);
                new_addresses.addresses[used_blocks as usize] = 0;
            }
        }

        Ok(new_addresses)
    }

    /// allocate a block
    ///
    /// # Returns
    /// the block's address
    fn allocate_block(&mut self) -> Option<usize> {
        let mut address = self.allocate(self.disk_parts.block_bit_map)?;

        // get physical address of the occupied block
        address *= BLOCK_SIZE;
        address += self.disk_parts.data;

        Some(address)
    }

    /// deallocate a block
    ///
    /// # Arguments
    /// - `address` - the block's address
    fn deallocate_block(&mut self, address: usize) {
        let block_number: usize =
            ((address - self.disk_parts.data / BLOCK_SIZE) as isize).abs() as usize;

        self.deallocate(self.disk_parts.block_bit_map, block_number);
    }

    /// function that adds a file to a folder
    ///
    /// # Arguments
    /// - `file` - the file that has to be added to the folder
    /// - `folder` - the folder that `file` is going to be added to
    ///
    /// # Returns
    /// Inode of the folder after the file was added or WriteError otherwise
    fn add_file_to_folder(
        &mut self,
        file: &DirEntry,
        folder: &mut Inode,
    ) -> Result<Inode, WriteError> {
        let buffer: &[u8] = unsafe {
            slice::from_raw_parts(file as *const _ as *const u8, core::mem::size_of_val(file))
        };

        unsafe { self.write(folder.id, buffer, folder.size) }
    }

    /// function that removes a file from a folder
    ///
    /// # Arguments
    /// - `file` - the file that has to be removed from the folder
    /// - `folder` - the folder that `file` is going to be removed from
    ///
    /// # Returns
    /// Inode of the folder after the file was removed or WriteError otherwise
    fn remove_file_from_folder(
        &mut self,
        file: &DirEntry,
        folder: &mut Inode,
    ) -> Result<Inode, WriteError> {
        let file_size = core::mem::size_of::<DirEntry>();
        let mut buffer: Vec<u8> = vec![0; file_size];
        let mut offset = 0;

        while offset < folder.size {
            unsafe { self.read(folder.id, buffer.as_mut_slice(), offset) };
            if unsafe {
                buffer.as_slice()
                    == slice::from_raw_parts(file as *const _ as *mut u8, buffer.len())
            } {
                break;
            }
            offset += file_size;
        }

        unsafe { self.read(folder.id, buffer.as_mut_slice(), folder.size - file_size) };

        let res = unsafe { self.write(folder.id, buffer.as_slice(), offset) };
        self.set_len(folder.id, folder.size - buffer.len());
        res
    }

    /// Calculate the disk parts for the file system.
    /// # Arguments
    /// - `device_size` - the disk device size.
    /// # Returns
    ///  a struct with pointers to every segment.
    fn calc_parts(device_size: usize) -> DiskParts {
        let mut parts: DiskParts = DiskParts {
            block_bit_map: 0,
            inode_bit_map: 0,
            root: 0,
            unused: 0,
            data: 0,
        };

        let mut remaining_space: usize = device_size - core::mem::size_of::<Header>();
        let mut amount_of_blocks: usize = remaining_space / BLOCK_SIZE;
        let mut amount_of_inodes: usize = 0;

        parts.block_bit_map = core::mem::size_of::<Header>();
        parts.inode_bit_map = parts.block_bit_map;

        while (parts.inode_bit_map - parts.block_bit_map) * BITS_IN_BYTE < amount_of_blocks {
            if (parts.inode_bit_map - parts.block_bit_map) % BLOCK_SIZE == 0 {
                amount_of_blocks -= 1;
            }
            parts.inode_bit_map += 1;
        }

        remaining_space = device_size - parts.inode_bit_map;
        amount_of_inodes = remaining_space / BYTES_PER_INODE;
        parts.root =
            parts.inode_bit_map + ((amount_of_inodes as f64 / BITS_IN_BYTE as f64) as usize + 1);
        parts.unused = parts.root + amount_of_inodes * core::mem::size_of::<Inode>();

        parts.data = parts.unused + (device_size - parts.unused) % BLOCK_SIZE;

        parts
    }

    /// Returns the `index`th pointer of the inode or `Err` if the `index` exceeds the maximum
    /// file size divided by the block size.
    ///
    /// # Arguments
    /// - `file` - The inode to get the pointer from.
    /// - `index` - The index of the pointer.
    fn get_ptr(&self, file: &Inode, index: usize) -> Result<usize, ()> {
        let offset;
        let mut ptr: usize = 0;

        if index < DIRECT_POINTERS {
            return Ok(file.addresses[index]);
        }

        offset = (index - DIRECT_POINTERS) * POINTER_SIZE;
        if offset > BLOCK_SIZE {
            return Err(());
        }
        unsafe {
            self.blkdev.read(
                file.indirect_pointer + offset,
                POINTER_SIZE,
                &mut ptr as *mut _ as *mut u8,
            );
        }

        Ok(ptr)
    }

    /// Set the value of the `index`th pointer.
    ///
    /// # Arguments
    /// - `file` - The file to change.
    /// - `index` - The index of the pointer.
    /// - `value` - The value to change to.
    ///
    /// # Returns
    /// `Err` if the pointer exceeds the maximum file size
    /// divided by the block size and `Ok` otherwise.
    fn set_ptr(&mut self, file: &mut Inode, index: usize, value: usize) -> Result<(), ()> {
        let offset;

        if index < DIRECT_POINTERS {
            file.addresses[index] = value;

            return Ok(());
        }

        offset = (index - DIRECT_POINTERS) * POINTER_SIZE;
        if offset > BLOCK_SIZE {
            return Err(());
        }
        if file.indirect_pointer == 0 {
            if let Some(indirect_pointer) = self.allocate_block() {
                file.indirect_pointer = indirect_pointer;
            } else {
                return Err(());
            }
        }
        unsafe {
            self.blkdev.write(
                file.indirect_pointer + offset,
                POINTER_SIZE,
                &value as *const _ as *const u8,
            );
        };

        Ok(())
    }

    /// Add the "." and ".." special folders to a folder.
    ///
    /// # Arguments
    /// - `containing_folder` - The folder that contains `folder`.
    /// - `folder` - The folder to add to.
    fn add_special_folders(&mut self, containing_folder: &Inode, folder: &mut Inode) {
        let dot = DirEntry {
            name: ".",
            id: folder.id,
        };
        let dot_dot = DirEntry {
            name: "..",
            id: containing_folder.id,
        };

        *folder = self.add_file_to_folder(&dot, folder).unwrap();
        self.add_file_to_folder(&dot_dot, folder);
    }

    /// function that checks if an inode is directory
    ///
    /// # Arguments
    ///  - `id` - the id of the Inode
    ///
    /// # Returns
    /// `true` if the inode is directory and `false` if not
    fn is_dir(&self, id: usize) -> bool {
        if let Some(inode) = self.read_inode(id) {
            inode.directory
        } else {
            false
        }
    }
}

/// public functions
impl Fs {
    /// funciton that creates new file system
    ///
    /// # Arguments
    /// - `blkdev` - the block device
    ///
    /// # Returns
    /// the file system
    pub fn new(blkdev: BlkDev) -> Self {
        let mut header = Header {
            magic: [0; 4],
            version: 0,
        };
        let mut instance;

        unsafe {
            blkdev.read(
                0,
                core::mem::size_of::<Header>(),
                &mut header as *mut Header as *mut u8,
            )
        };
        instance = Self {
            blkdev,
            disk_parts: Self::calc_parts(BlkDev::DEVICE_SIZE),
        };
        if header.magic != FS_MAGIC || header.version != CURR_VERSION {
            instance.format();
        }

        instance
    }

    /// format method
    /// This function discards the current content in the blockdevice and
    /// create a fresh new MYFS instance in the blockdevice.
    pub fn format(&mut self) {
        let mut header: Header = Header {
            magic: [0; 4],
            version: 0,
        };
        let bit_maps_size = self.disk_parts.root - self.disk_parts.block_bit_map;
        let zeroes_buf = vec![0; bit_maps_size];
        let mut root = Inode::new();

        // put the header in place
        header.magic.copy_from_slice(&FS_MAGIC);
        header.version = CURR_VERSION;
        unsafe {
            self.blkdev.write(
                0,
                core::mem::size_of_val(&header),
                &header as *const _ as *mut u8,
            )
        };

        // zero out bit maps
        unsafe {
            self.blkdev.write(
                self.disk_parts.block_bit_map,
                bit_maps_size,
                zeroes_buf.as_ptr() as *mut u8,
            )
        };

        // create root directory Inode
        root.directory = true;
        // UNWRAP: No inodes have been allocated yet.
        root.id = self.allocate_inode().unwrap();
        unsafe {
            self.blkdev.write(
                self.disk_parts.root,
                core::mem::size_of_val(&root),
                &root as *const _ as *mut u8,
            )
        };
        self.add_special_folders(&root.clone(), &mut root);
    }

    #[deprecated]
    pub fn create_file(&mut self, path_str: String, directory: bool) -> Result<(), &'static str> {
        let last_delimeter = if path_str.rfind('/').is_some() {
            path_str.rfind('/').unwrap()
        } else {
            0
        };
        let file_name = path_str[last_delimeter + 1..].to_string();
        let mut file = Inode::new();
        let mut dir;
        if let Some(inode) = self.get_inode(&path_str[0..(last_delimeter + 1)], None) {
            dir = inode
        } else {
            return Err("Error: invalid path");
        }
        let mut file_details = DirEntry { name: "", id: 0 };

        file.id = self.allocate_inode().unwrap();
        file.directory = directory;
        self.write_inode(&file);
        if file.directory {
            self.add_special_folders(&dir, &mut file)
        }

        file_details.name = Box::leak(file_name.into_boxed_str());
        file_details.id = file.id;
        match self.add_file_to_folder(&file_details, &mut dir) {
            Ok(_) => Ok(()),
            Err(_) => Err("Error: failed to add the file to the folder"),
        }
    }

    /// function that removes a file
    ///
    /// # Arguments
    /// - `path_str` - the path to the file
    /// - `directory` - if the file is a directory
    ///
    /// # Returns
    /// `Ok(())` if the file was removed successfully and `Err` if not
    pub fn remove_file(&mut self, path_str: String, directory: bool) -> Result<(), &'static str> {
        let last_delimeter = path_str.rfind('/').unwrap_or(0);
        let file_name = path_str[last_delimeter + 1..].to_string();
        let mut dir;
        if let Some(inode) = self.get_inode(&path_str[0..(last_delimeter + 1)], None) {
            dir = inode
        } else {
            return Err("Error: invalid path");
        }

        let mut file_details = DirEntry { name: "", id: 0 };
        // check if file exists
        if self.get_file_id(&path_str, None).is_none() {
            return Err("Error: file does not exist");
        }
        file_details.name = Box::leak(file_name.into_boxed_str());
        file_details.id = self.get_file_id(&path_str, None).unwrap();
        // the size of empty folder is 48
        if directory == true && self.read_inode(file_details.id).unwrap().size > 48 {
            return Err("Error: folder is not empty");
        } else if directory == false && self.is_dir(file_details.id) == true {
            return Err("Error: rm is used for file, not for directories");
        } else if directory == true && self.is_dir(file_details.id) == false {
            return Err("Error: rmdir is used for directories, not for files");
        }

        match self.remove_file_from_folder(&file_details, &mut dir) {
            Ok(_) => Ok(()),
            Err(_) => Err("Error: failed to remove the file from the folder"),
        }
    }

    /// Get a file's `Inode` id.
    ///
    /// # Arugments
    /// - `path` - The path to the file.
    /// - `cwd` - The current working directory, used for relative paths.
    pub fn get_file_id(&self, path: &str, cwd: Option<usize>) -> Option<usize> {
        Some(
            self.get_inode(
                path,
                if let Some(cwd) = cwd {
                    self.read_inode(cwd)
                } else {
                    None
                },
            )?
            .id,
        )
    }

    /// Read a file.
    ///
    /// # Arguments
    /// - `file` - The file's id.
    /// - `buffer` - The buffer to read into.
    /// - `offset` - The offset inside the file to read into.
    ///
    /// # Returns
    /// The amount of bytes read or `None` if the file does not exist.
    pub unsafe fn read(&self, file: usize, buffer: &mut [u8], offset: usize) -> Option<usize> {
        let inode = self.read_inode(file)?;
        let mut start = offset % BLOCK_SIZE;
        let mut to_read = BLOCK_SIZE - start;
        let mut pointer = offset / BLOCK_SIZE;
        let mut bytes_read = 0;
        let mut remaining;

        if offset >= inode.size {
            return Some(0);
        }

        remaining = if buffer.len() > inode.size - offset {
            inode.size - offset
        } else {
            buffer.len()
        };
        if to_read > remaining {
            to_read = remaining;
        }
        while remaining != 0 {
            // If there is no pointer read null bytes
            if self.get_ptr(&inode, pointer).unwrap() == 0 {
                for i in &mut buffer[(bytes_read + start)..(bytes_read + to_read)] {
                    *i = 0;
                }
            } else {
                self.blkdev.read(
                    self.get_ptr(&inode, pointer).unwrap() + start,
                    to_read,
                    buffer.as_mut_ptr().add(bytes_read),
                );
            }
            start = 0;
            bytes_read += to_read;
            remaining -= to_read;
            pointer += 1;
            to_read = if remaining < BLOCK_SIZE {
                remaining
            } else {
                BLOCK_SIZE
            };
        }

        Some(bytes_read)
    }

    /// Change the length of a file to a specific length.
    /// If the file has been set to a greater length, reading the extra data will return null bytes
    /// until the data is being written.
    /// If the file has been set to a smaller length, the extra data will be lost.
    ///
    /// # Arguments
    /// `file` - The `Inode` of the file.
    /// `size` - The required size.
    ///
    /// # Returns
    /// If the function fails, an error will be returned.
    pub fn set_len(&mut self, file: usize, size: usize) -> Result<(), SetLenError> {
        let mut resized;
        let last_ptr;
        let resized_last_ptr;
        let mut current;
        let mut block;

        if let Some(inode) = self.read_inode(file) {
            resized = inode;
        } else {
            return Err(SetLenError::FileNotFound);
        }
        if size > MAX_FILE_SIZE {
            return Err(SetLenError::MaximumSizeExceeded);
        }

        last_ptr = resized.size / BLOCK_SIZE;
        resized_last_ptr = size / BLOCK_SIZE;
        current = last_ptr;
        resized.size = size;
        // If the file has been resized to a smaller size, deallocate the unused blocks.
        while current > resized_last_ptr {
            block = self.get_ptr(&resized, current).unwrap();

            if block != 0 {
                self.deallocate_block(block);
                self.set_ptr(&mut resized, current, 0).unwrap();
            }
            current -= 1;
        }
        self.write_inode(&resized);

        Ok(())
    }

    /// Write data to a file.
    ///
    /// # Arguments
    /// - `file` - The `Inode` of the file.
    /// - `buffer` - A buffer containing the data to be written.
    /// - `offset` - The offset where the data will be written in the file.
    /// If the offset is at the end of the file or the data after it is written overflows the file's
    /// length the file will be extended.
    /// If the offset is beyond the file's size the file will be extended and a "hole" will be
    /// created in the file. Reading from the hole will return null bytes.
    ///
    /// # Returns
    /// if the function succeeded, If it fails, an error will be returned.
    pub unsafe fn write(
        &mut self,
        file: usize,
        buffer: &[u8],
        offset: usize,
    ) -> Result<Inode, WriteError> {
        let mut updated;
        let mut start = offset % BLOCK_SIZE;
        let mut to_write = BLOCK_SIZE - start;
        let mut pointer = offset / BLOCK_SIZE;
        let mut written = 0;
        let mut remaining = buffer.len();

        if let Some(inode) = self.read_inode(file) {
            updated = inode;
        } else {
            return Err(WriteError::FileNotFound);
        }
        if offset + remaining > updated.size {
            match self.set_len(file, offset + remaining) {
                // UNWRAP: We already checked if the file exists.
                Ok(_) => updated = self.read_inode(file).unwrap(),
                Err(SetLenError::MaximumSizeExceeded) => {
                    return Err(WriteError::MaximumSizeExceeded)
                }
                Err(SetLenError::FileNotFound) => return Err(WriteError::FileNotFound),
            }
        }

        if to_write > remaining {
            to_write = remaining
        }
        while remaining != 0 {
            if self.get_ptr(&updated, pointer).unwrap() == 0 {
                if let Some(block) = self.allocate_block() {
                    self.set_ptr(&mut updated, pointer, block).unwrap();
                } else {
                    return Err(WriteError::NotEnoughDiskSpace);
                }
            }
            self.blkdev.write(
                self.get_ptr(&updated, pointer).unwrap() + start,
                to_write,
                buffer.as_ptr().add(written),
            );
            written += to_write;
            remaining -= to_write;
            pointer += 1;
            to_write = if remaining < BLOCK_SIZE {
                remaining
            } else {
                BLOCK_SIZE
            };
            start = 0;
        }
        self.write_inode(&updated);

        Ok(updated)
    }

    /// function that returns the content of a file
    ///
    /// # Arguments
    /// - `path_str` - the path to the file
    ///
    /// # Returns
    /// the content if exists, None if not
    pub fn get_content(&mut self, path_str: &String) -> Option<String> {
        let file: Inode = self.get_inode(path_str, None)?;
        let mut content: Vec<u8> = vec![0; file.size];
        unsafe { self.read(file.id, content.as_mut_slice(), 0) };

        let content = String::from_utf8_lossy(&*content.as_slice()).to_string();
        if content.trim().is_empty() {
            None
        } else {
            Some(content)
        }
    }

    /// a function that list all the dirs (ls command)
    ///
    /// # Arguments
    /// - `path_str` - the path that need to be listed
    ///
    /// # Returns
    /// list with all the dirs and files
    pub fn list_dir(&self, path_str: &String) -> DirList {
        let mut ans: DirList = vec![];
        let mut entry: &mut DirListEntry = &mut DirListEntry {
            name: "",
            is_dir: false,
            file_size: 0,
        };
        let dir = self.get_inode(path_str, None).unwrap();
        let mut data: Vec<u8> = vec![0; dir.size];
        unsafe { self.read(dir.id, data.as_mut_slice(), 0) };
        let dir_content = unsafe {
            Box::from(slice::from_raw_parts(
                data.as_ptr() as *const DirEntry,
                data.len() / core::mem::size_of::<DirEntry>(),
            ))
        };
        let file = Inode::new();

        for i in 0..dir_content.len() {
            entry.name = dir_content[i].name;
            unsafe {
                self.blkdev.read(
                    self.get_inode_address(dir_content[i].id),
                    core::mem::size_of::<Inode>(),
                    &file as *const _ as *mut u8,
                )
            };
            entry.file_size = file.size;
            entry.is_dir = file.directory;
            ans.push(entry.clone());
        }

        ans
    }

    /// set the content of a file
    ///
    /// # Arguments
    /// - `path_str` - The path of the file
    /// - `content` - The new content of the file
    ///
    /// # Returns
    /// If the function fails, an error will be returned.
    pub fn set_content(
        &mut self,
        path_str: &String,
        content: &mut String,
    ) -> Result<(), &'static str> {
        let new_size: usize = content.len();
        let str_as_bytes: &mut [u8] = unsafe { content.as_bytes_mut() };
        let file: Inode;

        if let Some(f) = self.get_inode(path_str, None) {
            file = f;
        } else {
            return Err("Error: could not find the file");
        };

        self.set_len(file.id, new_size)
            .expect("Error: could not reallocate the block");

        if let Err(_) = unsafe { self.write(file.id, str_as_bytes, 0) } {
            return Err("Error: couldn't write to the file");
        }

        Ok(())
    }
}
