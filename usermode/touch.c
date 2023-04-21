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

int main(int argc, char** argv)
{
    if (argc <= 1)
    {
        print_str("touch: missing file operand\n"
                  "Usage: touch <file>\n");

        return 1;
    }
    if (creat(argv[1], FALSE) == -1)
    {
        print_str("touch: failed to create file\n");

        return 1;
    }

    return 0;
}
