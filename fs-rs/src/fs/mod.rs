use core::{
    borrow::{Borrow, BorrowMut},
    ptr::slice_from_raw_parts_mut,
    str::FromStr,
};
type DirList = Vec<DirListEntry>;

pub mod blkdev;
extern crate alloc;
const MAGIC_FS: [char; 4] = ['M', 'Y', 'F', 'S'];

pub(self) struct Inode {
    id: isize, // inode id
    directory: bool,
    size: usize,
    addresses: [isize; Constants::DirectPointers as usize],
}

#[derive(Clone)]
pub struct DirListEntry {
    name: alloc::string::String,

    is_dir: bool,

    file_size: isize,
}

pub(self) struct Header {
    magic: [char; 4],
    version: u8,
}

pub(self) struct DiskParts {
    blockBitMap: isize,
    inodeBitMap: isize,
    root: isize,
    unused: isize,
    data: isize,
}

pub(self) enum Constants {
    DirectPointers = 12,
    FileNameLen = 11,
    BlockSize = 16,
    BitsInByte = 8,
    BytesPerInode = 16 * 1024,
}

pub(self) struct DirEntry<'a> {
    name: &'a str,
    id: isize, // inode id
}

pub struct Fs {
    _parts: DiskParts,
    _device: blkdev::BlkDev,
}

impl core::clone::Clone for DiskParts {
    fn clone(&self) -> Self {
        Self {
            blockBitMap: self.blockBitMap,
            inodeBitMap: self.inodeBitMap,
            root: self.root,
            unused: self.unused,
            data: self.data,
        }
    }
}

impl Copy for DiskParts {}

impl core::clone::Clone for Fs {
    fn clone(&self) -> Self {
        Self {
            _parts: self._parts,
            _device: self._device.clone(),
        }
    }
}

impl core::clone::Clone for Inode {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            directory: self.directory,
            size: self.size,
            addresses: self.addresses,
        }
    }
}

// publics
impl Fs {
    const CURR_VERSION: u8 = 0x03;

    pub fn new(device: blkdev::BlkDev) -> Self {
        Self {
            _parts: Self::_calcParts(),
            _device: device,
        }
    }

    pub fn format(&mut self) {
        let mut header: Header = Header {
            magic: [0 as char; 4],
            version: 0,
        };

        let mut bitMapsSize: isize = self._parts.root - self._parts.blockBitMap;
        let mut zeroesBuf: Vec<u8> = vec![0; bitMapsSize as usize];
        let mut root: Inode = Inode {
            id: 0,
            directory: false,
            size: 0,
            addresses: [0; Constants::DirectPointers as usize],
        };

        header.magic = MAGIC_FS;
        header.version = Self::CURR_VERSION;
        unsafe {
            self._device.write(
                0,
                core::mem::size_of_val(&header),
                &header as *const _ as *mut u8,
            )
        };
        unsafe {
            self._device.write(
                self._parts.blockBitMap,
                bitMapsSize as usize,
                &zeroesBuf as *const _ as *mut u8,
            )
        };
        drop(zeroesBuf);

        root.directory = true;
        root.id = self._allocateInode();
        unsafe {
            self._device.write(
                self._parts.root,
                core::mem::size_of_val(&root),
                &root as *const _ as *mut u8,
            )
        };
    }

    /**
     * create_file method
     * Creates a new file in the required path.
     * @param path_str the file path (e.g. "/newfile")
     * @param directory boolean indicating whether this is a file or directory
     */
    pub fn create_file(&mut self, path_str: &str, directory: bool) {
        let LAST_DELIMITER: usize = path_str.rfind('/').unwrap();
        let FILE_NAME: &str = &path_str[(LAST_DELIMITER + 1)..];
        let mut file: Inode = Inode {
            id: 0,
            directory: false,
            size: 0,
            addresses: [0; Constants::DirectPointers as usize],
        };
        let mut dir: Inode = self._getInode(&path_str[0..LAST_DELIMITER]);
        let mut fileDetails: DirEntry = DirEntry { name: "", id: 0 };

        file.id = self._allocateInode();
        file.directory = directory;
        unsafe { self._writeInode(&file) };

        unsafe {
            core::ptr::copy_nonoverlapping(
                FILE_NAME as *const _ as *const (),
                &(fileDetails.name) as *const _ as *mut (),
                core::mem::size_of_val(fileDetails.name) - 1,
            )
        };
        fileDetails.id = file.id;
        self._addFileToFolder(&fileDetails, &mut dir);
    }

    /**
     * get_content method
     * Returns the whole content of the file indicated by path_str param.
     * Note: this method assumes path_str refers to a file and not a
     * directory.
     * @param path_str the file path (e.g. "/somefile")
     * @return the content of the file
     */
    pub fn get_content(&mut self, path_str: &str) -> alloc::string::String {
        let FILE: Inode = self._getInode(path_str);
        let temp: Vec<u8> = self._readInodeData(&FILE);
        let content: Vec<char> = temp.iter().map(|c| *c as char).collect();
        let contentStr: alloc::string::String = content.iter().collect::<String>();

        contentStr
    }

    /**
     * set_content method
     * Sets the whole content of the file indicated by path_str param.
     * Note: this method assumes path_str refers to a file and not a
     * directory.
     * @param path_str the file path (e.g. "/somefile")
     * @param content the file content string
     */
    pub fn set_content(&mut self, path_str: &str, content: &str) {
        let NEW_SIZE: usize = content.len();
        let LAST_POINTER: usize = NEW_SIZE / Constants::BlockSize as usize;
        let mut file: Inode = self._getInode(path_str);
        let mut pointer: usize = 0;
        let mut toWrite: isize = 0;
        let mut bytesWritten: usize = 0;

        file = self._reallocateBlocks(file, NEW_SIZE);
        file.size = NEW_SIZE;
        while bytesWritten != file.size {
            toWrite = if pointer == LAST_POINTER {
                (file.size % Constants::BitsInByte as usize) as isize
            } else {
                Constants::BlockSize as isize
            };

            unsafe {
                self._device.write(
                    file.addresses[pointer],
                    toWrite as usize,
                    (content as *const _ as *mut u8).add(bytesWritten),
                )
            };
            bytesWritten += toWrite as usize;
            pointer += 1;
        }
        unsafe { self._writeInode(&file) };
    }

    /**
     * list_dir method
     * Returns a list of a files in a directory.
     * Note: this method assumes path_str refers to a directory and not a
     * file.
     * @param path_str the file path (e.g. "/somedir")
     * @return a vector of dir_list_entry structures, one for each file in
     *	the directory.
     */
    pub fn list_dir(&mut self, path_str: &str) -> DirList {
        let mut ans: DirList = vec![];
        let mut entry: DirListEntry = DirListEntry {
            name: alloc::string::String::new(),
            is_dir: false,
            file_size: 0,
        };
        let DIR: Inode = self._getInode(path_str);
        let inodeData = self._readInodeData(&DIR);
        let mut rootDirContent: *mut DirEntry = inodeData.as_ptr() as *mut DirEntry;
        let mut file: Inode = Inode {
            id: 0,
            directory: false,
            size: 0,
            addresses: [0; Constants::DirectPointers as usize],
        };
        let mut dirEntries: usize = 0;

        dirEntries = DIR.size / core::mem::size_of::<DirEntry>();
        for i in 0..dirEntries {
            unsafe {
                entry.name = alloc::string::String::from_str(
                    (*slice_from_raw_parts_mut(rootDirContent, inodeData.len()))[i].name,
                )
                .unwrap();
                self._device.read(
                    self._getInodeAddress(
                        (*slice_from_raw_parts_mut(rootDirContent, inodeData.len()))[i].id,
                    ),
                    core::mem::size_of::<Inode>(),
                    &file as *const _ as *mut u8,
                );
            }
            entry.file_size = file.size as isize;
            entry.is_dir = file.directory;
            ans.push(entry);
        }
        drop(rootDirContent);

        ans
    }
}

// private
impl Fs {
    fn _getInodeAddress(self, id: isize) -> isize {
        return self._parts.root + id * core::mem::size_of::<Inode>() as isize;
    }

    fn _getRootDir(&mut self) -> Inode {
        let mut ans: Inode = Inode {
            id: 0,
            directory: false,
            size: 0,
            addresses: [0; 12],
        };

        unsafe {
            self._device.read(
                self._parts.root,
                core::mem::size_of::<Inode>(),
                &ans as *const _ as *mut u8,
            )
        };

        return ans;
    }

    fn _getInode(&mut self, mut path: &str) -> Inode {
        let mut nextDelimiter: Option<usize> = path.find('/');
        let mut nextFolder: alloc::vec::Vec<char>;
        let mut inode: Inode = self._getRootDir();
        let mut dirContent: *mut DirEntry = core::ptr::null_mut();
        let mut index: usize = 0;

        if path == "/" {
            return inode; // return root dir
        }

        while nextDelimiter != None {
            dirContent = self._readInodeData(&inode).as_ptr() as *mut DirEntry;
            path = &path[(nextDelimiter.unwrap() + 1)..];
            nextDelimiter = path.find('/');
            nextFolder = path
                .chars()
                .take(nextDelimiter.unwrap())
                .collect::<alloc::vec::Vec<char>>()
                .to_vec();
            while unsafe {
                (*dirContent.add(index))
                    .name
                    .chars()
                    .collect::<alloc::vec::Vec<char>>()
                    .to_vec()
                    != nextFolder
            } {
                index += 1;
            }
            unsafe {
                self._device.read(
                    self._getInodeAddress((*dirContent.add(index)).id),
                    core::mem::size_of::<Inode>(),
                    &inode as *const _ as *mut u8,
                );
                index = 0
            };
        }

        inode
    }

    fn _readInodeData(&mut self, inode: &Inode) -> Vec<u8> {
        let LAST_POINTER: usize = inode.size / Constants::BlockSize as usize;
        let mut pointer: usize = 0;
        let mut toRead: isize = 0;
        let buffer: alloc::vec::Vec<u8> = alloc::vec![0; inode.size];
        let mut bytesRead: usize = 0;

        while bytesRead != inode.size {
            toRead = if pointer == LAST_POINTER {
                (inode.size % (Constants::BlockSize as usize)) as isize
            } else {
                Constants::BlockSize as isize
            };
            unsafe {
                self._device.read(
                    inode.addresses[pointer],
                    toRead as usize,
                    (buffer.as_ptr() as *mut u8).add(bytesRead),
                )
            };
            bytesRead += toRead as usize;
            pointer += 1;
        }

        buffer
    }

    unsafe fn _writeInode(&mut self, inode: &Inode) {
        self._device.write(
            self._getInodeAddress(inode.id),
            core::mem::size_of_val(inode),
            inode as *const _ as *mut u8,
        );
    }

    fn _addFileToFolder(&mut self, file: &DirEntry, folder: &mut Inode) {
        let mut pointer: usize = folder.size / Constants::BlockSize as usize;
        let mut bytesLeft: isize = core::mem::size_of_val(file) as isize;
        let mut address: isize = 0;
        let mut spaceTakenInLastBlock: isize =
            (folder.size % Constants::BlockSize as usize) as isize;
        let mut emptySpace: isize = 0;
        let mut toWrite: isize = 0;
        let mut written: usize = 0;

        *folder =
            self._reallocateBlocks(folder.clone(), folder.size + core::mem::size_of_val(file));
        address = folder.addresses[pointer] + spaceTakenInLastBlock;
        emptySpace = Constants::BlockSize as isize - spaceTakenInLastBlock;
        toWrite = if bytesLeft > emptySpace {
            emptySpace
        } else {
            bytesLeft
        };

        loop {
            unsafe {
                self._device.write(
                    address,
                    toWrite as usize,
                    (file as *const _ as *mut u8).add(written),
                )
            };

            folder.size += toWrite as usize;
            written += toWrite as usize;
            bytesLeft -= toWrite;
            pointer += 1;
            address = folder.addresses[pointer];

            // setup for next iteration
            toWrite = if bytesLeft > Constants::BlockSize as isize {
                Constants::BlockSize as isize
            } else {
                bytesLeft
            };

            if bytesLeft == 0 {
                break;
            }
        }

        unsafe { self._writeInode(folder) };
    }

    fn _allocate(&mut self, bitmapStart: isize) -> isize {
        /*

        return address;
            */

        let BITS_IN_BUFFER: isize = 64;
        let BYTES_IN_BUFFER: isize = BITS_IN_BUFFER / Constants::BitsInByte as isize;
        let ALL_OCCUPIED: usize = 0xFFFFFFFFFFFFFFFF;
        let mut buffer: usize = 0;
        let mut address: isize = bitmapStart;

        loop {
            unsafe {
                self._device.read(
                    address,
                    Constants::BitsInByte as usize,
                    &buffer as *const _ as *mut _,
                )
            };
            address += BYTES_IN_BUFFER;
            if buffer != ALL_OCCUPIED {
                break;
            }
        }

        address -= BYTES_IN_BUFFER;

        for i in 0..BYTES_IN_BUFFER {
            if ((buffer & (1 << i)) == 0)
            // if the (i)'s bit is 0
            {
                buffer ^= 1 << i; // flip the bit to mark as occupied
                unsafe {
                    self._device.write(
                        address,
                        BYTES_IN_BUFFER as usize,
                        &buffer as *const _ as *mut u8,
                    )
                };

                // get the index in the bitmap
                address -= bitmapStart;
                address += i;

                // once we found unoccupied space, we finished our task
                break;
            }
        }

        address
    }

    fn _deallocate(&mut self, bitmapStart: isize, n: isize) {
        let mut byteAddress: isize = bitmapStart + n / Constants::BitsInByte as isize;
        let mut byte: isize = 0;
        let mut offset: usize = n as usize % Constants::BitsInByte as usize;

        unsafe {
            self._device
                .read(byteAddress, 1, &byte as *const _ as *mut u8)
        };
        byte ^= 1 << offset; // flip the bit to mark as unoccupied
        unsafe {
            self._device
                .write(byteAddress, 1, &byte as *const _ as *mut u8)
        };
    }

    fn _reallocateBlocks(&mut self, inode: Inode, newSize: usize) -> Inode {
        let mut i: isize = 0;
        let mut usedBlocks: isize = 0;
        let mut requiredBlocks: usize = newSize / Constants::BlockSize as usize
            + ((newSize % Constants::BlockSize as usize != 0) as usize);
        let mut blocksToAllocate: isize = 0;
        let mut newAddresses: Inode = inode.clone();

        assert!(
            !(requiredBlocks > Constants::DirectPointers as usize),
            "Error: reached maximum file size"
        );

        while newAddresses.addresses[usedBlocks as usize] != 0 {
            usedBlocks += 1;
        }

        blocksToAllocate = (requiredBlocks - usedBlocks as usize) as isize;

        if blocksToAllocate > 0 {
            for i in 0..blocksToAllocate {
                newAddresses.addresses[usedBlocks as usize] = self._allocateBlock();
                usedBlocks += 1;
            }
        } else if blocksToAllocate < 0 {
            for i in 0..-blocksToAllocate {
                usedBlocks -= 1;
                self._deallocateBlock(newAddresses.addresses[usedBlocks as usize]);
                newAddresses.addresses[usedBlocks as usize] = 0;
            }
        }

        newAddresses
    }

    fn _allocateInode(&mut self) -> isize {
        self._allocate(self._parts.inodeBitMap)
    }

    fn _allocateBlock(&mut self) -> isize {
        let mut address: isize = self._allocate(self._parts.inodeBitMap);

        // get physical address of the occupied block
        address *= Constants::BlockSize as isize;
        address += self._parts.data;

        address
    }

    fn _deallocateBlock(&mut self, address: isize) {
        let blockNumber: isize = (address - self._parts.data) / Constants::BlockSize as isize;

        let bitmap = self._parts.blockBitMap;

        self._deallocate(bitmap, blockNumber);
    }

    fn _calcParts() -> DiskParts {
        use micromath::F32;

        let deviceSize: isize = blkdev::BlkDev::DEVICE_SIZE as isize;
        let mut parts: DiskParts = DiskParts {
            blockBitMap: 0,
            inodeBitMap: 0,
            root: 0,
            unused: 0,
            data: 0,
        };
        let mut remainingSpace: isize = deviceSize - core::mem::size_of::<Header>() as isize;
        let mut amountOfBlocks: isize = remainingSpace / Constants::BlockSize as isize;
        let mut amountOfInodes: isize = 0;

        parts.blockBitMap = core::mem::size_of::<Header>() as isize;
        parts.inodeBitMap = parts.blockBitMap;

        while (parts.inodeBitMap - parts.blockBitMap) * (Constants::BitsInByte as isize)
            < amountOfBlocks
        {
            if (parts.inodeBitMap - parts.blockBitMap) % (Constants::BlockSize as isize) == 0 {
                amountOfBlocks -= 1;
            }
            parts.inodeBitMap += 1;
        }

        remainingSpace = deviceSize - parts.inodeBitMap;
        amountOfInodes = remainingSpace / Constants::BytesPerInode as isize;
        let float_amount_of_inodes: F32 = F32(amountOfInodes as f32);
        let float_bits_in_bytes: F32 = F32(Constants::BitsInByte as usize as f32);
        parts.root =
            parts.inodeBitMap + ((float_amount_of_inodes / float_bits_in_bytes).ceil().0 as isize);
        parts.unused = parts.root + amountOfInodes * (core::mem::size_of::<Inode>() as isize);

        parts.data = parts.unused + (deviceSize - parts.unused) % Constants::BlockSize as isize;

        parts
    }
}
