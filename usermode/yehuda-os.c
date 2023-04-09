#include "yehuda-os.h"

const size_t READ        = 0x0;
const size_t WRITE       = 0x1;
const size_t OPEN        = 0x2;
const size_t FSTAT       = 0x5;
const size_t MALLOC      = 0x9;
const size_t FREE        = 0xb;
const size_t EXEC        = 0x3b;
const size_t EXIT        = 0x3c;
const size_t FCHDIR      = 0x51;
const size_t CREAT       = 0x55;
const size_t REMOVE_FILE = 0x57;
const size_t READ_DIR    = 0x59;
const size_t TRUNCATE    = 0x4c;
const size_t FTRUNCATE   = 0x4d;

size_t
syscall(size_t syscall_number, size_t arg0, size_t arg1, size_t arg2, size_t arg3, size_t arg4, size_t arg5)
{
    size_t result;
    register long r10 asm("r10") = arg3;
    register long r8 asm("r10")  = arg4;
    register long r9 asm("r10")  = arg5;

    asm volatile("syscall" ::"%rax"(syscall_number), "%rdi"(arg0), "%rsi"(arg1),
    "%rdx"(arg2), "r"(r10), "r"(r8), "r"(r9));
    asm("movq %%rax, %0;" : "=r"(result));

    return result;
}

