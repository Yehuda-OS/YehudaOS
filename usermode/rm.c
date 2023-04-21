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
        print_str("rm: missing file operand");
        print_newline();
        print_str("Usage: rm <file>");
        print_newline();
    }
    char *slash = strrchr(argv[1], '/');
    int fd = open(argv[1]);

    if (fd != -1)
    {
        if (slash != NULL)
        {
            int idx = (int)(slash - argv[1]);
            char path[idx + 2];
            strncpy(path, argv[1], idx + 1);
            if (open(path) == -1)
            {
                print_str("invalid path");
                print_newline();
                return 0;
            }
        }
        struct Stat stat = {.directory = 0, .size = 0};
        fstat(fd, &stat);
        if (stat.directory == TRUE)
        {
            print_str("Error: Cannot remove folders using 'rm' command. Only files can be deleted.");
            print_newline();
            return 1;
        }
    }

    remove_file(argv[1]);

    return 0;
}
