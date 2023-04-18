#include "yehuda-os/sys.h"
#include "yehuda-os/helpers.h"

int list_dir(const char *path)
{
    int fd = open(path);
    struct DirEntry *dirent;

    while (readdir(fd, 0, dirent) != -1)
    {
        print_str(dirent->name);
    }

    return 0;
}

int main(int argc, char **argv)
{
    return list_dir(argc == 0 ? "." : argv[0]);
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