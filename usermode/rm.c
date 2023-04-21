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
        print_str("rm: missing file operand");
        print_newline();
        print_str("Usage: rm <file>");
        print_newline();

        return 1;
    }
    if (remove_file(argv[1]) == -1)
    {
        print_str("rm: cannot remove file/directory\n");

        return 1;
    }

    return 0;
}
