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
        print_str("touch: missing file operand");
        print_newline();
        print_str("Usage: touch <file>");
        print_newline();
    }
    int count = 0, idx = 0;
    char *str = argv[1];
    while (*str)
    { // loop until end of string
        if (*str == '/')
        {            // check if current character is a '/'
            count++; // increment count if it is
        }
        str++; // move to the next character in the string
        idx++;
    }

    if (count > 1)
    {
        char path[idx];
        for (int i = 0; i < idx; i++)
        {
            path[i] = argv[1][i];
        }

        if (open(path) == -1)
        {
            print_str("invalid path");
            print_newline();
            return 0;
        }
    }
    creat(argv[1], FALSE);

    return 0;
}
