#include "yehuda-os/helpers.h"
#include "yehuda-os/sys.h"

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

int main(int argc, char **argv)
{
    if (argc < 2)
    {
        print_str("rmdir: missing dir operand");
        print_newline();
        print_str("Usage: rmdir <dir_name>");
        print_newline();
    }
    char *slash = strrchr(argv[1], '/');
    int fd = open(argv[1]);

    if (fd != -1)
    {
        struct Stat stat = {.directory = 0, .size = 0};
        fstat(fd, &stat);
        if (stat.directory == FALSE)
        {
            print_str("Error: Cannot remove files using 'rmdir' command. Only folders can be deleted.");
            print_newline();
            return 1;
        }
    }

    remove_file(argv[1]);

    return 0;
}
