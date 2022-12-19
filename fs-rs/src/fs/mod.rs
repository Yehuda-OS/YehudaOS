pub mod blkdev;

extern crate alloc;

use alloc::boxed::Box;
use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};
use blkdev::BlkDev;
use core::option::Option::None;
use core::result::{Result, Result::Err, Result::Ok};
use core::slice;

pub type DirList = Vec<DirListEntry>;

const FS_MAGIC: [u8; 4] = *b"FSRS";
const CURR_VERSION: u8 = 0x1;

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

const DIRECT_POINTERS: usize = 12;
const FILE_NAME_LEN: usize = 11;
const BLOCK_SIZE: usize = 16;
const BITS_IN_BYTE: usize = 8;
const BYTES_PER_INODE: usize = 16 * 1024;

#[derive(Clone, Copy)]
struct Inode {
    id: usize,
    directory: bool,
    size: usize,
    addresses: [usize; DIRECT_POINTERS],
}

#[derive(Clone)]
pub struct DirListEntry {
    pub name: String,
    pub is_dir: bool,
    pub file_size: usize,
}

#[derive(Clone)]
struct DirEntry {
    name: String,
    id: usize,
}

/// private functions
impl Fs {
    fn get_root_dir(&self) -> Inode {
        let mut ans = Inode {
            id: 0,
            directory: false,
            size: 0,
            addresses: [0; DIRECT_POINTERS],
        };

        unsafe {
            self.blkdev.read(
                self.disk_parts.root,
                core::mem::size_of::<Inode>(),
                &mut ans as *mut Inode as *mut u8,
            )
        };

        ans
    }

    fn get_inode(&self, mut path: String) -> Inode {
        let mut next_delimiter = path.find('/');
        let mut next_folder = String::new();
        let mut inode = self.get_root_dir();
        let mut dir_content;
        let mut index: usize = 0;

        if path == "/" {
            return inode;
        }

        while next_delimiter != None {
            dir_content = self.read_dir(&inode);
            path = path[(next_delimiter.unwrap() + 1)..].to_string();
            next_delimiter = path.find('/');
            next_folder = path[0..next_delimiter.unwrap()].to_string();
            while dir_content[index].name != next_folder {
                index += 1;
            }

            unsafe {
                self.blkdev.read(
                    self.get_inode_address(dir_content[index].id),
                    core::mem::size_of::<Inode>(),
                    &mut inode as *mut Inode as *mut u8,
                )
            };
            index = 0;
        }

        inode
    }

    fn get_inode_address(&self, id: usize) -> usize {
        self.disk_parts.root + id * core::mem::size_of::<Inode>()
    }

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

    fn read_dir(&self, inode: &Inode) -> Box<&[DirEntry]> {
        let data = self.read_file(inode);

        unsafe {
            Box::from(slice::from_raw_parts(
                data.as_ptr() as *const DirEntry,
                data.len() / core::mem::size_of::<DirEntry>(),
            ))
        }
    }

    fn write_inode(&mut self, inode: &Inode) {
        unsafe {
            self.blkdev.write(
                self.get_inode_address(inode.id),
                core::mem::size_of::<Inode>(),
                inode as *const _ as *mut u8,
            )
        };
    }

    fn allocate_inode(&mut self) -> usize {
        self.allocate(self.disk_parts.inode_bit_map)
    }

    fn allocate(&mut self, bitmap_start: usize) -> usize {
        const BITS_IN_BUFFER: usize = 64;
        const BYTES_IN_BUFFER: usize = BITS_IN_BUFFER / BITS_IN_BYTE;
        const ALL_OCCUPIED: usize = 0xFFFFFFFFFFFFFFFF;
        let mut buffer: usize = 0;
        let mut address: usize = bitmap_start;

        // read the bitmap until an unoccupied memory is found
        loop {
            unsafe {
                self.blkdev.read(
                    address,
                    BYTES_IN_BUFFER,
                    core::mem::transmute(&buffer as *const usize as *mut u8),
                )
            };
            address += BYTES_IN_BUFFER;
            if buffer != ALL_OCCUPIED {
                break;
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
        address
    }

    fn deallocate(&mut self, bitmap_start: usize, n: usize) {
        let byte_address: usize = bitmap_start + n / BITS_IN_BYTE;
        let mut byte: usize = 0;
        let mut offset: usize = n % BITS_IN_BYTE;

        unsafe {
            self.blkdev
                .read(byte_address, 1, &byte as *const usize as *mut u8)
        };
        byte ^= 1 << offset; // flip the bit to mark as unoccupied
        unsafe {
            self.blkdev
                .write(byte_address, 1, &byte as *const usize as *mut u8)
        };
    }

    fn reallocate_blocks(&mut self, inode: &Inode, new_size: usize) -> Result<Inode, &'static str> {
        let mut used_blocks: isize = 0;
        let required_blocks: usize = new_size / BLOCK_SIZE + (new_size % BLOCK_SIZE != 0) as usize;
        let mut blocks_to_allocate: isize = 0;
        let mut new_addresses: Inode = *inode;

        if required_blocks > DIRECT_POINTERS {
            return Err("too many blocks are required");
        }

        while new_addresses.addresses[used_blocks as usize] != 0 {
            used_blocks += 1;
        }

        blocks_to_allocate = (required_blocks as isize - used_blocks) as isize;

        if blocks_to_allocate > 0 {
            for i in 0..blocks_to_allocate {
                new_addresses.addresses[used_blocks as usize] = self.allocate_block();
                used_blocks += 1;
            }
        } else if blocks_to_allocate < 0 {
            for i in 0..(-blocks_to_allocate) {
                used_blocks -= 1;
                self.deallocate_block(new_addresses.addresses[used_blocks as usize]);
                new_addresses.addresses[used_blocks as usize] = 0;
            }
        }

        Ok(new_addresses)
    }

    fn allocate_block(&mut self) -> usize {
        let mut address: usize = self.allocate(self.disk_parts.block_bit_map);

        // get physical address of the occupied block
        address *= BLOCK_SIZE;
        address += self.disk_parts.data;

        address
    }

    fn deallocate_block(&mut self, address: usize) {
        let block_number: usize = (address - self.disk_parts.data) / BLOCK_SIZE;

        self.deallocate(self.disk_parts.block_bit_map, block_number);
    }

    fn add_file_to_folder(&mut self, file: &DirEntry, folder: &mut Inode) {
        let mut pointer = folder.size / BLOCK_SIZE;
        let mut bytes_left = core::mem::size_of_val(file);
        let mut address;
        let space_taken_in_last_block = folder.size % BLOCK_SIZE;
        let empty_space;
        let mut to_write;
        let mut written = 0;

        *folder = self
            .reallocate_blocks(folder, folder.size + core::mem::size_of_val(file))
            .unwrap();

        address = folder.addresses[pointer] + space_taken_in_last_block;
        empty_space = BLOCK_SIZE - space_taken_in_last_block;
        to_write = if bytes_left > empty_space {
            empty_space
        } else {
            bytes_left
        };

        loop {
            unsafe {
                self.blkdev.write(
                    address,
                    to_write,
                    (file as *const _ as *mut u8).add(written),
                )
            };

            folder.size += to_write;
            written += to_write;
            bytes_left -= to_write;
            pointer += 1;
            address = folder.addresses[pointer];

            to_write = if bytes_left > BLOCK_SIZE {
                BLOCK_SIZE
            } else {
                bytes_left
            };

            if bytes_left == 0 {
                break;
            }
        }
        self.write_inode(folder);
    }

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

    /// Add the "." and ".." special folders to a folder.
    ///
    /// # Arguments
    /// - `containing_folder` - The folder that contains `folder`.
    /// - `folder` - The folder to add to.
    fn add_special_folders(&mut self, containing_folder: &Inode, folder: &mut Inode) {
        let dot = DirEntry {
            name: String::from("."),
            id: folder.id,
        };
        let dot_dot = DirEntry {
            name: String::from(".."),
            id: containing_folder.id,
        };

        self.add_file_to_folder(&dot, folder);
        self.add_file_to_folder(&dot_dot, folder);
    }
}

/// public functions
impl Fs {
    pub fn new(mut blkdev: BlkDev) -> Self {
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

    pub fn format(&mut self) {
        let mut header: Header = Header {
            magic: [0; 4],
            version: 0,
        };

        let bit_maps_size = self.disk_parts.root - self.disk_parts.block_bit_map;
        let zeroes_buf = vec![0; bit_maps_size];
        let mut root: Inode = Inode {
            id: 0,
            directory: false,
            size: 0,
            addresses: [0; DIRECT_POINTERS],
        };

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
        root.id = self.allocate_inode();
        unsafe {
            self.blkdev.write(
                self.disk_parts.root,
                core::mem::size_of_val(&root),
                &root as *const _ as *mut u8,
            )
        };
        self.add_special_folders(&root.clone(), &mut root);
    }

    pub fn create_file(&mut self, path_str: String, directory: bool) {
        let last_delimeter = if path_str.rfind('/').is_some() {
            path_str.rfind('/').unwrap()
        } else {
            0
        };
        let file_name = path_str[last_delimeter + 1..].to_string();
        let mut file: Inode = Inode {
            id: 0,
            directory: false,
            size: 0,
            addresses: [0; DIRECT_POINTERS],
        };
        let mut dir = self.get_inode(path_str[0..(last_delimeter + 1)].to_string());
        let mut file_details = DirEntry {
            name: "".to_string(),
            id: 0,
        };

        file.id = self.allocate_inode();
        file.directory = directory;
        self.write_inode(&file);
        if file.directory {
            self.add_special_folders(&dir, &mut file)
        }

        file_details.name = file_name;
        file_details.id = file.id;
        self.add_file_to_folder(&file_details, &mut dir);
    }

    pub unsafe fn read(&self, file: &str, buffer: &mut [u8], offset: usize) -> usize {
        let inode = self.get_inode(file.to_string());
        let mut start = offset % BLOCK_SIZE;
        let mut to_read = BLOCK_SIZE - start;
        let mut pointer = offset / BLOCK_SIZE;
        let mut bytes_read = 0;
        let mut remaining;

        if offset >= inode.size {
            return 0;
        }

        remaining = if buffer.len() > inode.size - offset {
            inode.size - offset
        } else {
            buffer.len()
        };
        if remaining < to_read {
            to_read = remaining;
        }
        while remaining != 0 {
            self.blkdev.read(
                inode.addresses[pointer] + start,
                to_read,
                buffer.as_mut_ptr().add(bytes_read),
            );
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

        bytes_read
    }

    pub fn get_content(&mut self, path_str: &String) -> String {
        let file: Inode = self.get_inode(path_str.clone());
        let content = self.read_file(&file);

        String::from_utf8_lossy(*content).to_string()
    }

    pub fn list_dir(&self, path_str: &String) -> DirList {
        let mut ans: DirList = vec![];
        let mut entry: &mut DirListEntry = &mut DirListEntry {
            name: "".to_string(),
            is_dir: false,
            file_size: 0,
        };
        let dir = self.get_inode(path_str.clone());
        let dir_content = self.read_dir(&dir);
        let file = Inode {
            id: 0,
            directory: false,
            size: 0,
            addresses: [0; DIRECT_POINTERS],
        };

        for i in 0..dir_content.len() {
            entry.name = dir_content[i].name.clone();
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

    pub fn set_content(&mut self, path_str: &String, content: &String) {
        let new_size: usize = content.len();
        let last_pointer: usize = new_size / BLOCK_SIZE;
        let mut str_as_arr: Vec<char> = content.chars().collect();
        let mut file: Inode = self.get_inode(path_str.clone());
        let mut pointer: usize = 0;
        let mut to_write: usize = 0;
        let mut bytes_written: usize = 0;

        file = self
            .reallocate_blocks(&file, new_size)
            .expect("Error: could not reallocate the block");
        file.size = new_size;
        while bytes_written != file.size {
            to_write = if pointer == last_pointer {
                file.size % BLOCK_SIZE
            } else {
                BLOCK_SIZE
            };
            unsafe {
                self.blkdev.write(
                    file.addresses[pointer],
                    to_write,
                    str_as_arr.as_mut_ptr().add(bytes_written) as *mut u8,
                )
            };
            bytes_written += to_write;
            pointer += 1;
        }

        self.write_inode(&file);
    }
}
