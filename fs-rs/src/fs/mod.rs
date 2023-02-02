mod blkdev;
mod inode;

extern crate alloc;

use alloc::boxed::Box;
use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::option::Option::None;
use core::result::{Result, Result::Err, Result::Ok};
use core::slice;
use inode::Inode;

pub type DirList = Vec<DirListEntry>;

const FS_MAGIC: [u8; 4] = *b"FSRS";
const CURR_VERSION: u8 = 0x1;
const FILE_NAME_LEN: usize = 11;
const BLOCK_SIZE: usize = 4096;
const BITS_IN_BYTE: usize = 8;
const BYTES_PER_INODE: usize = 16 * 1024;
const DISK_PARTS: DiskParts = calc_parts(blkdev::DEVICE_SIZE);

#[derive(Debug)]
pub enum FsError {
    NotEnoughDiskSpace,
    MaximumSizeExceeded,
    FileNotFound,
}

struct Header {
    magic: [u8; 4],
    version: u8,
}

#[derive(Clone, Copy)]
struct DiskParts {
    block_bit_map: usize,
    inode_bit_map: usize,
    root: usize,
    unused: usize,
    data: usize,
}

#[derive(Clone)]
pub struct DirListEntry {
    pub name: &'static str,
    pub is_dir: bool,
    pub file_size: usize,
}

#[derive(Clone, PartialEq, Eq)]
struct DirEntry {
    name: [u8; FILE_NAME_LEN],
    id: usize,
}

/// function that returns the root dir (/)
///
/// # Returns
/// the Inode of the root dir
fn get_root_dir() -> Inode {
    let mut ans = Inode::new();

    unsafe {
        blkdev::read(
            DISK_PARTS.root,
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
fn get_inode(mut path: &str, cwd: Option<Inode>) -> Option<Inode> {
    let mut next_delimiter = path.find('/');
    let mut next_folder;
    let mut inode = get_root_dir();
    let mut index: usize = 0;

    if path == "/" {
        return Some(inode);
    }
    // Check if the path is relative
    if path.chars().nth(0).unwrap_or(' ') != '/' {
        inode = cwd?;
    }

    while next_delimiter != None {
        let mut data: Vec<u8> = vec![0; inode.size()];
        let dir_content;
        unsafe { read(inode.id, data.as_mut_slice(), index) };

        dir_content = unsafe {
            core::slice::from_raw_parts(
                data.as_ptr() as *const DirEntry,
                data.len() / core::mem::size_of::<DirEntry>(),
            )
        };

        path = &path[(next_delimiter.unwrap() + 1)..];
        next_delimiter = path.find('/');
        next_folder = match next_delimiter {
            Some(delimeter) => &path[0..delimeter],
            None => &path[0..],
        };
        while index != dir_content.len()
            && core::str::from_utf8(
                &dir_content[index].name[..dir_content[index]
                    .name
                    .iter()
                    .position(|x| *x == 0)
                    .unwrap_or(FILE_NAME_LEN)],
            )
            .unwrap()
                != next_folder
        {
            index += 1;
        }
        if index >= dir_content.len() {
            return None;
        }
        match read_inode(dir_content[index].id) {
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
fn get_inode_address(id: usize) -> usize {
    DISK_PARTS.root + id * core::mem::size_of::<Inode>()
}

#[deprecated]
fn read_file(inode: &Inode) -> Box<&[u8]> {
    let last_pointer: usize = inode.size() / BLOCK_SIZE;
    let mut pointer = 0;
    let mut to_read;
    let mut bytes_read = 0;
    let mut buffer = Box::new(vec![0; inode.size()]);

    while bytes_read != inode.size() {
        to_read = if pointer == last_pointer {
            inode.size() % BLOCK_SIZE
        } else {
            BLOCK_SIZE
        };
        unsafe {
            blkdev::read(
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
fn read_dir(inode: &Inode) -> Box<&[DirEntry]> {
    let data = read_file(inode);

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
fn is_allocated(bitmap_start: usize, i: usize) -> bool {
    let byte_address = bitmap_start + i / BITS_IN_BYTE;
    let offset = i % BITS_IN_BYTE;
    let mut byte: u8 = 0;

    unsafe { blkdev::read(byte_address, 1, &mut byte as *mut u8) }

    byte & (1 << offset) != 0
}

/// Returns the `Inode` object with a specific ID, or None if the inode is not
/// associated with any file.
///
/// # Arguments
/// - `id` - The inode's ID.
fn read_inode(id: usize) -> Option<Inode> {
    let mut inode = Inode::new();

    if is_allocated(DISK_PARTS.inode_bit_map, id) {
        unsafe {
            blkdev::read(
                get_inode_address(id),
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
fn write_inode(inode: &Inode) {
    unsafe {
        blkdev::write(
            get_inode_address(inode.id),
            core::mem::size_of::<Inode>(),
            inode as *const _ as *mut u8,
        )
    };
}

/// allocate Inode
///
/// # Returns
/// the address of the inode if it was allocated or None if no free space was found
fn allocate_inode() -> Option<usize> {
    allocate(DISK_PARTS.inode_bit_map, DISK_PARTS.root)
}

/// allocate a block or Inode
///
/// # Arguments
/// - `bitmap_start` - the start of the bitmap
///
/// # Returns
/// the address of the allocated block or Inode
fn allocate(bitmap_start: usize, bitmap_end: usize) -> Option<usize> {
    const BITS_IN_BUFFER: usize = 64;
    const BYTES_IN_BUFFER: usize = BITS_IN_BUFFER / BITS_IN_BYTE;
    const ALL_OCCUPIED: usize = !0;
    let mut buffer: usize = ALL_OCCUPIED;
    let mut address: usize = bitmap_start;

    // read the bitmap until unoccupied memory is found
    while buffer == ALL_OCCUPIED {
        unsafe { blkdev::read(address, BYTES_IN_BUFFER, &mut buffer as *mut _ as *mut u8) };
        address += BYTES_IN_BUFFER;
        if address >= bitmap_end {
            // Force the bits that are outside of the bitmap to 1.
            buffer |= !(!0 << (address - bitmap_end) * BITS_IN_BYTE);

            if buffer == ALL_OCCUPIED {
                return None;
            }
        }
    }
    address -= BYTES_IN_BUFFER;

    // read the buffer until an unoccupied memory is found
    for i in 0..BITS_IN_BUFFER {
        // if the (i)'s bit is 0
        if buffer & (1 << i) == 0 {
            buffer ^= 1 << i; // flip the bit to mark as occupied
            unsafe {
                blkdev::write(
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
fn deallocate(bitmap_start: usize, n: usize) {
    let byte_address: usize = bitmap_start + n / BITS_IN_BYTE;
    let mut byte: u8 = 0;
    let offset = n % BITS_IN_BYTE;

    unsafe { blkdev::read(byte_address, 1, &mut byte as *mut u8) };
    byte ^= 1 << offset; // flip the bit to mark as unoccupied
    unsafe { blkdev::write(byte_address, 1, &mut byte as *mut u8) };
}

#[deprecated]
fn reallocate_blocks(inode: &Inode, new_size: usize) -> Result<Inode, &'static str> {
    let required_blocks = new_size / BLOCK_SIZE + (new_size % BLOCK_SIZE != 0) as usize;
    let blocks_to_allocate;
    let mut used_blocks = 0;
    let mut new_addresses = *inode;

    if required_blocks > inode::DIRECT_POINTERS {
        return Err("too many blocks are required");
    }

    while new_addresses.addresses[used_blocks as usize] != 0 {
        used_blocks += 1;
    }

    blocks_to_allocate = (required_blocks as isize - used_blocks) as isize;

    if blocks_to_allocate > 0 {
        for _ in 0..blocks_to_allocate {
            new_addresses.addresses[used_blocks as usize] = allocate_block().unwrap();
            used_blocks += 1;
        }
    } else if blocks_to_allocate < 0 {
        for _ in 0..(-blocks_to_allocate) {
            used_blocks -= 1;
            deallocate_block(new_addresses.addresses[used_blocks as usize]);
            new_addresses.addresses[used_blocks as usize] = 0;
        }
    }

    Ok(new_addresses)
}

/// allocate a block
///
/// # Returns
/// the block's address
fn allocate_block() -> Option<usize> {
    let mut address = allocate(DISK_PARTS.block_bit_map, DISK_PARTS.inode_bit_map)?;

    // get physical address of the occupied block
    address *= BLOCK_SIZE;
    address += DISK_PARTS.data;

    Some(address)
}

/// deallocate a block
///
/// # Arguments
/// - `address` - the block's address
fn deallocate_block(address: usize) {
    let block_number = (address - DISK_PARTS.data) / BLOCK_SIZE;

    deallocate(DISK_PARTS.block_bit_map, block_number);
}

/// function that adds a file to a folder
///
/// # Arguments
/// - `file` - the file that has to be added to the folder
/// - `folder` - the folder that `file` is going to be added to
///
/// # Returns
/// Inode of the folder after the file was added or `FsError` otherwise
fn add_file_to_folder(file: &DirEntry, folder: usize) -> Result<(), FsError> {
    let folder_size = read_inode(folder).ok_or(FsError::FileNotFound)?.size();
    let buffer: &[u8] = unsafe {
        slice::from_raw_parts(file as *const _ as *const u8, core::mem::size_of_val(file))
    };

    unsafe { write(folder, buffer, folder_size) }
}

/// function that removes a file from a folder
///
/// # Arguments
/// - `file` - The id of the file that has to be removed from the folder.
/// - `folder` - The id of the folder that `file` is going to be removed from.
///
/// # Returns
/// `FileNotFound` error if the folder does not exist or the file is
/// not inside the folder, `Ok` otherwise.
fn remove_file_from_folder(file: usize, folder: usize) -> Result<(), FsError> {
    let file_size = core::mem::size_of::<DirEntry>();
    let mut buffer: Vec<u8> = vec![0; file_size];
    let mut offset = 0;
    let folder_size = read_inode(folder).ok_or(FsError::FileNotFound)?.size();

    loop {
        // UNWRAP: We already checked if the folder exists.
        if unsafe { read(folder, buffer.as_mut_slice(), offset).unwrap() } == 0 {
            return Err(FsError::FileNotFound);
        }
        if unsafe { (*(buffer.as_ptr() as *const DirEntry)).id == file } {
            break;
        }
        offset += file_size;
    }

    unsafe {
        read(folder, buffer.as_mut_slice(), folder_size - file_size);
        // UNWRAP: We already checked if the folder exists and we write inside the folder where
        // there was already data.
        write(folder, buffer.as_slice(), offset).unwrap();
    };
    // UNWRAP: We already checked if the folder exists and we shrink the folder, thus we can't
    // exceed the maximum file size.
    set_len(folder, folder_size - buffer.len()).unwrap();

    Ok(())
}

/// Calculate the disk parts for the file system.
/// # Arguments
/// - `device_size` - the disk device size.
/// # Returns
///  a struct with pointers to every segment.
const fn calc_parts(device_size: usize) -> DiskParts {
    let mut parts: DiskParts = DiskParts {
        block_bit_map: 0,
        inode_bit_map: 0,
        root: 0,
        unused: 0,
        data: 0,
    };

    let mut remaining_space: usize = device_size - core::mem::size_of::<Header>();
    let mut amount_of_blocks: usize = remaining_space / BLOCK_SIZE;
    let amount_of_inodes: usize;

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
    parts.root = parts.inode_bit_map + ((amount_of_inodes / BITS_IN_BYTE) + 1);
    parts.unused = parts.root + amount_of_inodes * core::mem::size_of::<Inode>();

    parts.data = parts.unused + (device_size - parts.unused) % BLOCK_SIZE;

    parts
}

/// Add the "." and ".." special folders to a folder.
///
/// # Arguments
/// - `containing_folder` - The folder that contains `folder`.
/// - `folder` - The folder to add to.
fn add_special_folders(containing_folder: &Inode, folder: &mut Inode) {
    let dot = DirEntry {
        name: { ['.' as u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
        id: folder.id,
    };
    let dot_dot = DirEntry {
        name: ['.' as u8, '.' as u8, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        id: containing_folder.id,
    };

    add_file_to_folder(&dot, folder.id).unwrap();
    *folder = read_inode(folder.id).unwrap();
    add_file_to_folder(&dot_dot, folder.id).unwrap();
    *folder = read_inode(folder.id).unwrap();
}

/// function that checks if an inode is directory
///
/// # Arguments
///  - `id` - the id of the Inode
///
/// # Returns
/// `true` if the inode is directory and `false` if not
fn is_dir(id: usize) -> bool {
    if let Some(inode) = read_inode(id) {
        inode.directory
    } else {
        false
    }
}

/// Initialize the file system.
/// Must be called before performing any other operation.
///
/// # Arguments
/// - `blkdev` - the block device
pub fn init() {
    let mut header = Header {
        magic: [0; 4],
        version: 0,
    };

    blkdev::init();
    unsafe {
        blkdev::read(
            0,
            core::mem::size_of::<Header>(),
            &mut header as *mut Header as *mut u8,
        )
    };
    if header.magic != FS_MAGIC || header.version != CURR_VERSION {
        format();
    }
}

/// format method
/// This function discards the current content in the blockdevice and
/// create a fresh new MYFS instance in the blockdevice.
pub fn format() {
    let mut header: Header = Header {
        magic: [0; 4],
        version: 0,
    };
    let bit_maps_size = DISK_PARTS.root - DISK_PARTS.block_bit_map;
    let mut root = Inode::new();

    // put the header in place
    header.magic.copy_from_slice(&FS_MAGIC);
    header.version = CURR_VERSION;
    unsafe {
        blkdev::write(
            0,
            core::mem::size_of_val(&header),
            &header as *const _ as *mut u8,
        )
    };

    // zero out bit maps
    unsafe {
        blkdev::set(DISK_PARTS.block_bit_map, bit_maps_size, 0);
    };

    // create root directory Inode
    root.directory = true;
    // UNWRAP: No inodes have been allocated yet.
    root.id = allocate_inode().unwrap();
    unsafe {
        blkdev::write(
            DISK_PARTS.root,
            core::mem::size_of_val(&root),
            &root as *const _ as *mut u8,
        )
    };
    add_special_folders(&root.clone(), &mut root);
}

#[deprecated]
pub fn create_file(path_str: String, directory: bool) -> Result<(), &'static str> {
    let last_delimeter = path_str.rfind('/').unwrap_or(0);
    let file_name = path_str[last_delimeter + 1..].to_string();
    let mut file = Inode::new();
    let dir;
    if let Some(inode) = get_inode(&path_str[0..(last_delimeter + 1)], None) {
        dir = inode
    } else {
        return Err("Error: invalid path");
    }
    let mut file_details = DirEntry {
        name: [0; FILE_NAME_LEN],
        id: 0,
    };

    file.id = allocate_inode().unwrap();
    file.directory = directory;
    write_inode(&file);
    if file.directory {
        add_special_folders(&dir, &mut file)
    }

    file_details.name = {
        let mut name: [u8; FILE_NAME_LEN] = [0; FILE_NAME_LEN];
        let temp = file_name.as_bytes();
        if temp.len() >= FILE_NAME_LEN {
            name = temp[..FILE_NAME_LEN].try_into().unwrap();
        } else {
            for i in 0..temp.len() {
                name[i] = temp[i];
            }
        }

        name
    };
    file_details.id = file.id;
    match add_file_to_folder(&file_details, dir.id) {
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
pub fn remove_file(path_str: String, directory: bool) -> Result<(), &'static str> {
    let last_delimeter = path_str.rfind('/').unwrap_or(0);
    let file_name = path_str[last_delimeter + 1..].to_string();
    let dir;
    if let Some(inode) = get_inode(&path_str[0..(last_delimeter + 1)], None) {
        dir = inode
    } else {
        return Err("Error: invalid path");
    }

    let mut file_details = DirEntry {
        name: [0; FILE_NAME_LEN],
        id: 0,
    };
    // check if file exists
    if get_file_id(&path_str, None).is_none() {
        return Err("Error: file does not exist");
    }
    file_details.name = {
        let mut name: [u8; FILE_NAME_LEN] = [0; FILE_NAME_LEN];
        let temp = file_name.as_bytes();
        if temp.len() >= FILE_NAME_LEN {
            name = temp[..FILE_NAME_LEN].try_into().unwrap();
        } else {
            for i in 0..temp.len() {
                name[i] = temp[i];
            }
        }

        name
    };
    file_details.id = get_file_id(&path_str, None).unwrap();
    // An empty folder contains 2 entries
    if directory == true
        && read_inode(file_details.id).unwrap().size() > 2 * core::mem::size_of::<DirEntry>()
    {
        return Err("Error: folder is not empty");
    } else if directory == false && is_dir(file_details.id) == true {
        return Err("Error: rm is used for file, not for directories");
    } else if directory == true && is_dir(file_details.id) == false {
        return Err("Error: rmdir is used for directories, not for files");
    }
    remove_file_from_folder(file_details.id, dir.id)
        .map_err(|_| "Error: failed to remove the file from the folder")?;

    Ok(())
}

/// Get a file's `Inode` id.
///
/// # Arugments
/// - `path` - The path to the file.
/// - `cwd` - The current working directory, used for relative paths.
pub fn get_file_id(path: &str, cwd: Option<usize>) -> Option<usize> {
    Some(
        get_inode(
            path,
            if let Some(cwd) = cwd {
                read_inode(cwd)
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
pub unsafe fn read(file: usize, buffer: &mut [u8], offset: usize) -> Option<usize> {
    let inode = read_inode(file)?;
    let mut start = offset % BLOCK_SIZE;
    let mut to_read = BLOCK_SIZE - start;
    let mut pointer = offset / BLOCK_SIZE;
    let mut bytes_read = 0;
    let mut remaining;

    if offset >= inode.size() {
        return Some(0);
    }

    remaining = if buffer.len() > inode.size() - offset {
        inode.size() - offset
    } else {
        buffer.len()
    };
    if to_read > remaining {
        to_read = remaining;
    }
    while remaining != 0 {
        // If there is no pointer read null bytes
        if inode.get_ptr(pointer).unwrap() == 0 {
            for i in &mut buffer[(bytes_read + start)..(bytes_read + to_read)] {
                *i = 0;
            }
        } else {
            blkdev::read(
                inode.get_ptr(pointer).unwrap() + start,
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
/// - `file` - The `Inode` of the file.
/// - `size` - The required size.
///
/// # Returns
/// The function returns the `FileNotFound` or `MaximumSizeExceeded` error.
pub fn set_len(file: usize, size: usize) -> Result<(), FsError> {
    let last_ptr;
    let resized_last_ptr = size / BLOCK_SIZE;
    let mut current;
    let mut block;
    let mut resized = read_inode(file).ok_or(FsError::FileNotFound)?;

    last_ptr = resized.size() / BLOCK_SIZE;
    current = last_ptr;
    // If the file has been resized to a smaller size, deallocate the unused blocks.
    while current > resized_last_ptr {
        block = resized.get_ptr(current)?;

        if block != 0 {
            deallocate_block(block);
            // UNWRAP: The pointer is guarenteed to be inside the file and there is enough
            // space for the pointer because it is already occupied by `block`.
            resized.set_ptr(current, 0).unwrap();
        }
        current -= 1;
    }
    resized.set_size(size)?;
    write_inode(&resized);

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
/// If the function fails, an error will be returned.
pub unsafe fn write(file: usize, buffer: &[u8], offset: usize) -> Result<(), FsError> {
    let mut start = offset % BLOCK_SIZE;
    let mut to_write = BLOCK_SIZE - start;
    let mut pointer = offset / BLOCK_SIZE;
    let mut written = 0;
    let mut remaining = buffer.len();
    let mut updated = read_inode(file).ok_or(FsError::FileNotFound)?;

    if offset + remaining > updated.size() {
        // UNWRAP: We already checked if the file exists.
        set_len(file, offset + remaining).map(|_| updated = read_inode(file).unwrap())?;
    }

    if to_write > remaining {
        to_write = remaining
    }
    while remaining != 0 {
        // UNWRAP: The pointer is in the file's range because
        // we change the file's size accordingly.
        if updated.get_ptr(pointer).unwrap() == 0 {
            updated
                .set_ptr(
                    pointer,
                    allocate_block().ok_or(FsError::NotEnoughDiskSpace)?,
                )
                .unwrap();
        }
        blkdev::write(
            updated.get_ptr(pointer).unwrap() + start,
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
    write_inode(&updated);

    Ok(())
}

/// function that returns the content of a file
///
/// # Arguments
/// - `path_str` - the path to the file
///
/// # Returns
/// the content if exists, None if not
pub fn get_content(path_str: &String) -> Option<String> {
    let file: Inode = get_inode(path_str, None)?;
    let mut content: Vec<u8> = vec![0; file.size()];
    unsafe { read(file.id, content.as_mut_slice(), 0) };

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
pub fn list_dir(path_str: &String) -> DirList {
    let mut ans: DirList = vec![];
    let mut entry: &mut DirListEntry = &mut DirListEntry {
        name: "",
        is_dir: false,
        file_size: 0,
    };
    let dir = get_inode(path_str, None).unwrap();
    let mut data: Vec<u8> = vec![0; dir.size()];
    unsafe { read(dir.id, data.as_mut_slice(), 0) };
    let dir_content = unsafe {
        Box::from(slice::from_raw_parts(
            data.as_ptr() as *const DirEntry,
            data.len() / core::mem::size_of::<DirEntry>(),
        ))
    };
    let file = Inode::new();

    for i in 0..dir_content.len() {
        entry.name = Box::leak(
            String::from_utf8(dir_content[i].name.to_vec())
                .unwrap()
                .into_boxed_str(),
        );
        unsafe {
            blkdev::read(
                get_inode_address(dir_content[i].id),
                core::mem::size_of::<Inode>(),
                &file as *const _ as *mut u8,
            )
        };
        entry.file_size = file.size();
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
pub fn set_content(path_str: &String, content: &mut String) -> Result<(), &'static str> {
    let new_size: usize = content.len();
    let str_as_bytes: &mut [u8] = unsafe { content.as_bytes_mut() };
    let file: Inode;

    if let Some(f) = get_inode(path_str, None) {
        file = f;
    } else {
        return Err("Error: could not find the file");
    };

    set_len(file.id, new_size).expect("Error: could not reallocate the block");

    if let Err(_) = unsafe { write(file.id, str_as_bytes, 0) } {
        return Err("Error: couldn't write to the file");
    }

    Ok(())
}
