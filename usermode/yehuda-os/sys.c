#include "sys.h"

const size_t READ                 = 0x0;
const size_t WRITE                = 0x1;
const size_t OPEN                 = 0x2;
const size_t FSTAT                = 0x5;
const size_t WAITPID              = 0x7;
const size_t MALLOC               = 0x9;
const size_t CALLOC               = 0xa;
const size_t FREE                 = 0xb;
const size_t REALLOC              = 0xc;
const size_t EXEC                 = 0x3b;
const size_t EXIT                 = 0x3c;
const size_t GET_CURRENT_DIR_NAME = 0x4f;
const size_t CHDIR                = 0x50;
const size_t CREAT                = 0x55;
const size_t REMOVE_FILE          = 0x57;
const size_t READ_DIR             = 0x59;
const size_t TRUNCATE             = 0x4c;
const size_t FTRUNCATE            = 0x4d;

size_t
syscall(size_t syscall_number, size_t arg0, size_t arg1, size_t arg2, size_t arg3, size_t arg4, size_t arg5)
{
    size_t result;
    register size_t rax asm("rax") = syscall_number;
    register size_t rdi asm("rdi") = arg0;
    register size_t rsi asm("rsi") = arg1;
    register size_t rdx asm("rdx") = arg2;
    register size_t r10 asm("r10") = arg3;
    register size_t r8 asm("r8")   = arg4;
    register size_t r9 asm("r9")   = arg5;

    asm volatile("syscall" ::"r"(rax), "r"(rdi), "r"(rsi), "r"(rdx), "r"(r10),
    "r"(r8), "r"(r9));
    asm("movq %%rax, %0;" : "=r"(result));

    return result;
}

/**
 * Read bytes from a file descriptor.
 *
 * `fd`: The file descriptor to read from.
 * `buf`: The buffer to write into.
 * `count`: The number of bytes to read.
 * `offset`: The offset in the file to start reading from, ignored for `stdin`.
 *
 * returns: The amount of bytes read or -1 on failure.
 */
ssize_t read(int fd, void* buf, size_t count, size_t offset)
{
    return (ssize_t)syscall(READ, fd, (size_t)buf, count, offset, 0, 0);
}

/**
 * Write bytes to a file descriptor.
 *
 * `fd`: The file descriptor to write to.
 * `buf`: A buffer containing the data to be written.
 * `offset`: The offset where the data will be written in the file, this is ignored for `stdout`.
 *           If the offset is beyond the file's size the file will be extended and a "hole" will
 *           be created in the file.
 *           Reading from a hole will return null bytes.
 * returns: 0 if the operation was successful, -1 otherwise.
 */
int write(int fd, const void* buf, size_t count, size_t offset)
{
    return (int)syscall(WRITE, fd, (size_t)buf, count, offset, 0, 0);
}

/**
 * Get a file descriptor for a file.
 *
 * # Arguments
 * `pathname`: Path to the file.
 *
 * returns: The file descriptor for the file on success or -1 otherwise.
 */
int open(const char* pathname)
{
    return (int)syscall(OPEN, (size_t)pathname, 0, 0, 0, 0, 0);
}

/**
 * Get information about a file.
 *
 * `fd`: The file descriptor of that file.
 * `statbuf`: A buffer to the `Stat` struct that will contain the information about the file.
 * The struct contains the file's size or for directories the amount of files in the directory.
 *
 * returns: 0 if the file exists and -1 if it doesn't or if `fd` is negative.
 */
int fstat(int fd, struct Stat* statbuf)
{
    return (int)syscall(FSTAT, fd, (size_t)statbuf, 0, 0, 0, 0);
}

/**
 * Awaits the calling process until a specific process terminates.
 *
 * `pid`: The process ID of the process to wait for.
 *        Must be a non-negative number.
 * `wstatus`: A buffer to write the process' exit code into.
 *
 * returns: 0 on sucess or -1 on error.
 *          Possible errors:
 *          - `pid` is negative.
 *          - The process specified by `pid` does not exist.
 *          - The process specified by `pid` has already finished its execution.
 */
int waitpid(pid_t pid, int* wstatus)
{
    return (int)syscall(WAITPID, pid, (size_t)wstatus, 0, 0, 0, 0);
}

/**
 * Allocate memory for a userspace program.
 *
 * `size`: The size of the allocation.
 *
 * returns: Apointer to the allocation or null on failure.
 */
void* malloc(size_t size)
{
    return (void*)syscall(MALLOC, size, 0, 0, 0, 0, 0);
}

/**
 * Behaves like `malloc`, but sets the memory to 0.
 *
 * `nitems`: The number of elements to be allocated.
 * `size`: The size of each element.
 */
void* calloc(size_t nitems, size_t size)
{
    return (void*)syscall(CALLOC, nitems, size, 0, 0, 0, 0);
}

/**
 * Deallocate an allocation that was allocated with `malloc`.
 *
 * `ptr`: The pointer to the allocation that was returned from `malloc`.
 */
void free(void* ptr)
{
    syscall(FREE, (size_t)ptr, 0, 0, 0, 0, 0);
}

/**
 * Grow or shrink a block that was allocated with `malloc`.
 * Copies the data from the original block to the new block.
 *
 * `ptr`: The block that was allocated with `malloc`.
 *        If `ptr` is `NULL`, then the call is equivalent to `malloc(size)`
 * `size`: The new required size of the block.
 *
 * returns: A pointer to a new allocation or null on failure.
 */
void* realloc(void* ptr, size_t size)
{
    if (ptr == NULL)
    {
        return malloc(size);
    }

    return (void*)syscall(REALLOC, (size_t)ptr, size, 0, 0, 0, 0);
}

/**
 * Execute a program in a new process.
 *
 * `pathname`: Path to the file to execute, must be a valid ELF file.
 * `argv`: The commandline arguments.
 *
 * returns: The process ID of the new process if the operation was successful, -1 otherwise.
 */
int exec(const char* pathname, char* const argv[])
{
    return (int)syscall(EXEC, (size_t)pathname, (size_t)argv, 0, 0, 0, 0);
}

/**
 * Terminate the calling process.
 *
 * `status`: The exit code of the process.
 */
void exit(int status)
{
    syscall(EXIT, (size_t)status, 0, 0, 0, 0, 0);
    // `syscall` will never return when the `EXIT` code is passed.
    // Therefore we tell the compiler that any code after it is unreachable.
    __builtin_unreachable();
}

/**
 * Get the current working directory.
 *
 * returns: On success, a string containing the current working directory
 *          that has been allocated with `malloc` will be returned.
 *          It is the user's responsibility to free the buffer with `free`.
 *          On failure, `NULL` is returned.
 */
char* get_current_dir_name()
{
    return (char*)syscall(GET_CURRENT_DIR_NAME, 0, 0, 0, 0, 0, 0);
}

/**
 * Change the current working directory.
 *
 * `path`: Path to the new working directory.
 *
 * returns: 0 if the operation was successful or -1 on failure.
 *          Possible failures:
 *          - `path` is invalid.
 *          - `path` does not exist.
 *          - `path` is not a directory.
 */
int chdir(const char* path)
{
    return (int)syscall(CHDIR, (size_t)path, 0, 0, 0, 0, 0);
}

/**
 * Create a file in the file system.
 *
 * `path`: Path to the file.
 * `path_len`: Length of the path.
 * `directory`: Whether the new file should be a directory.
 *
 * returns: The file descriptor of the new file if the operation was successful, -1 otherwise.
 */
int creat(const char* path, bool_t directory)
{
    return (int)syscall(CREAT, (size_t)path, (size_t)directory, 0, 0, 0, 0);
}

/// Remove a file from the file system, or remove a directory that must be empty.
///
/// # Arguments
/// - `path` - Path to the file.
/// - `path_len` - Length of the path.
///
/// # Returns
/// 0 if the operation was successful, -1 otherwise.
int remove_file(const char* path)
{
    return (int)syscall(REMOVE_FILE, (size_t)path, 0, 0, 0, 0, 0);
}

/**
 * Read a directory entry.
 *
 * `fd`: The file descriptor of the directory.
 * `offset`: The offset **in files** inside the directory to read from.
 * `dirp`: A buffer to write the data into.
 *
 * returns: 0 on success, -1 on failure.
 *          Possible failures:
 *          - `fd` is negative or invalid.
 *          - `fd` is not a directory.
 */
int readdir(int fd, size_t offset, struct DirEntry* dirp)
{
    return (int)syscall(READ_DIR, fd, offset, (size_t)dirp, 0, 0, 0);
}

/**
 * Change the length of a file to a specific ljength.
 * If the file has been set to a greater length, reading the extra data will return null bytes
 * until the data is being written.
 * If the file has been set to a smaller length, the extra data will be lost.
 *
 * `path`: Path to the file.
 * `length`: The required size.
 *
 * returns: 0 if the operation was successful, -1 otherwise.j
 */
int truncate(const char* path, size_t length)
{
    return (int)syscall(TRUNCATE, (size_t)path, length, 0, 0, 0, 0);
}

/**
 * Change the length of a file to a specific length.
 * If the file has been set to a greater length, reading the extra data will return null bytes
 * until the data is being written.
 * If the file has been set to a smaller length, the extra data will be lost.
 *
 * `fd`: The file descriptor of the file.
 * `length`: The required size.
 *
 * returns: 0 if the operation was successful, -1 otherwise.
 */
int ftruncate(int fd, size_t length)
{
    return (int)syscall(FTRUNCATE, fd, length, 0, 0, 0, 0);
}
