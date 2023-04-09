#include "yehuda-os.h"

void syscall(size_t syscall_number, size_t arg0, size_t arg1, size_t arg2, size_t arg3, size_t arg4, size_t arg5)
{
    register long r10 asm("r10") = arg3;
    register long r8 asm("r10")  = arg4;
    register long r9 asm("r10")  = arg5;

    asm volatile("syscall" ::"%rax"(syscall_number), "%rdi"(arg0), "%rsi"(arg1),
    "%rdx"(arg2), "r"(r10), "r"(r8), "r"(r9));
}
