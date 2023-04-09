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

int read(int fd, void* buf, size_t count, size_t offset)
{
    return syscall(READ, fd, (size_t)buf, count, offset, 0, 0);
}

int write(int fd, const void* buf, size_t count, size_t offset)
{
    return syscall(WRITE, fd, (size_t)buf, count, offset, 0, 0);
}

int open(const char* pathname)
{
    return syscall(OPEN, (size_t)pathname, 0, 0, 0, 0, 0);
}

int fstat(int fd, struct Stat* statbuf)
{
    return syscall(FSTAT, fd, (size_t)statbuf, 0, 0, 0, 0);
}

void* malloc(size_t size)
{
    return (void*)syscall(MALLOC, size, 0, 0, 0, 0, 0);
}

void free(void* ptr)
{
    syscall(FREE, (size_t)ptr, 0, 0, 0, 0, 0);
}

int exec(const char* pathname)
{
    return syscall(EXEC, (size_t)pathname, 0, 0, 0, 0, 0);
}

void exit(int status)
{
    syscall(EXIT, (size_t)status, 0, 0, 0, 0, 0);
    // `syscall` will never return when the `EXIT` code is passed.
    // Therefore we put this while loop to suppress any warnings.
    while (1)
    {
    }
}

int fchdir(int fd)
{
    return syscall(FCHDIR, fd, 0, 0, 0, 0, 0);
}

int creat(const char* path, bool_t directory)
{
    return syscall(CREAT, (size_t)path, (size_t)directory, 0, 0, 0, 0);
}

int remove_file(const char* path)
{
    return syscall(REMOVE_FILE, (size_t)path, 0, 0, 0, 0, 0);
}

int readdir(int fd, size_t offset, struct DirEntry* dirp)
{
    return syscall(READ_DIR, fd, offset, (size_t)dirp, 0, 0, 0);
}

int truncate(const char* path, size_t length)
{
    return syscall(TRUNCATE, (size_t)path, length, 0, 0, 0, 0);
}

int ftruncate(int fd, size_t length)
{
    return syscall(FTRUNCATE, fd, length, 0, 0, 0, 0);
}
