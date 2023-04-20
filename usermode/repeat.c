#include "yehuda-os/helpers.h"

int main(int argc, char** argv)
{
    if (argc <= 1)
    {
        print_str("repeat: missing parameter to print\n");

        return 1;
    }

    for (int i = 0; i < 50; i++)
    {
        print_str(argv[1]);
    }
    print_newline();

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
