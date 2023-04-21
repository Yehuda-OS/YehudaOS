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
    if (argc == 1)
    {
        print_newline();
    }
    else
    {
        int len = 0;
        for (int i = 1; i < argc; i++)
        {
            len += strlen(argv[i]) + 1;
        }
        char* msg = (char*)calloc(len, sizeof(char));
        for (int i = 1; i < argc; i++)
        {
            strcat(msg, argv[i]);
            strcat(msg, " ");
        }
        print_str(msg);
        print_newline();
    }

    return 0;
}
