#include "yehuda-os/sys.h"
#include "yehuda-os/helpers.h"

#define BUF_SIZE 1024

int main(int argc, char *argv[])
{
    struct DirEntry *entry;

    int fd = open(argc == 0 ? "/." : argv[0]);
    int res = readdir(fd, 0, entry);
    print_str(entry->name);

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