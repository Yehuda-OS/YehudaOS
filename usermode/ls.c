#include "yehuda-os/sys.h"
#include "yehuda-os/helpers.h"

#define BUF_SIZE 1024

int main(int argc, char *argv[])
{
    // let mut ans: DirList = vec![];
    // let mut entry: &mut DirListEntry = &mut DirListEntry {
    //     name: "",
    //     is_dir: false,
    //     file_size: 0,
    // };

    // let dir = get_inode(path_str, None).unwrap();
    // let mut data: Vec<u8> = vec![0; dir.size()];
    // unsafe { read(dir.id(), data.as_mut_slice(), 0) };
    // let dir_content = unsafe {
    //     Box::from(slice::from_raw_parts(
    //         data.as_ptr() as *const DirEntry,
    //         data.len() / core::mem::size_of::<DirEntry>(),
    //     ))
    // };
    // let file = Inode::default();

    // for i in 0..dir_content.len() {
    //     entry.name = Box::leak(
    //         String::from_utf8(dir_content[i].name.to_vec())
    //             .unwrap()
    //             .into_boxed_str(),
    //     );
    //     unsafe {
    //         blkdev::read(
    //             get_inode_address(dir_content[i].id),
    //             core::mem::size_of::<Inode>(),
    //             &file as *const _ as *mut u8,
    //         )
    //     };
    //     entry.file_size = file.size();
    //     entry.is_dir = file.is_dir();
    //     ans.push(entry.clone());
    // }

    // ans
    int fd = open(argc == 1 ? "/." : argv[1]);
    struct Stat stat;
    fstat(fd, &stat);
    struct dirent *entry = {0, 0};

    readdir(fd, 0, entry);
    int offset = 0;
    for (; offset < stat.size; offset += sizeof(struct DirEntry))
    {
        print_str(((struct DirEntry *)((struct DirEntry *)entry + offset))->name);
        print_newline();
    }

    return 0;
}

// Tell the compiler incoming stack alignment is not RSP%16==8 or ESP%16==12
__attribute__((force_align_arg_pointer)) void _start()
{
    asm("call main");

    /* exit system call */
    asm("mov $0, %rdi;"
        "mov %eax, %edi;"
        "mov $0x3c, %rax;"
        "syscall");
    // tell the compiler to make sure side effects are done before the asm
    // statement
    __builtin_unreachable();
}