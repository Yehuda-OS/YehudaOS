#include "yehuda-os/helpers.h"
#include "yehuda-os/sys.h"

#define NUM_OF_PROCESSES 5

int main()
{
    int status                   = 0;
    pid_t pids[NUM_OF_PROCESSES] = { 0, 0, 0, 0, 0 };
    char* const args[]           = { "./repeat", "a", NULL };

    for (int i = 0; i < NUM_OF_PROCESSES; i++)
    {
        print_str("Creating process\n");
        pids[i] = exec("/repeat", args);
        if (pids[i] == -1)
        {
            print_str("execution of one of the processes failed\n");

            return 1;
        }
        args[1][0]++;
    }
    for (int i = 0; i < NUM_OF_PROCESSES; i++)
    {
        waitpid(pids[i], &status);
    }

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
