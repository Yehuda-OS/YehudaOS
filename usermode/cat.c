#include "yehuda-os/sys.h"
#include "yehuda-os/helpers.h"

int main(int argc, char **argv)
{
    int fd = open(argv[1]);
    if (fd == -1)
    {
        print_str("File does not exist.");
        print_newline();
        return 1;
    }
    struct Stat stat = {.directory = 0, .size = 0};
    fstat(fd, &stat);
    if (stat.directory == TRUE)
    {
        print_str("Specified path is not a file.");
        print_newline();
        return 1;
    }

    read return 0;
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
    // tell the compiler to make sure side effects are done before the asm statement
    __builtin_unreachable();
}
