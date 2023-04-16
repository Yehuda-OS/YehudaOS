#ifndef YEHUDAOS_SYS
#define YEHUDAOS_SYS

#include "defines.h"

typedef long pid_t;

struct Stat
{
    size_t size;
    bool_t directory;
};

struct DirEntry
{
    char name[FILE_NAME_LEN];
    size_t id;
};

ssize_t read(int fd, void* buf, size_t count, size_t offset);

int write(int fd, const void* buf, size_t count, size_t offset);

int open(const char* pathname);

int fstat(int fd, struct Stat* statbuf);

void* malloc(size_t size);

void* calloc(size_t nitems, size_t size);

void free(void* ptr);

void* realloc(void* ptr, size_t size);

int exec(const char* pathname);

void exit(int status);

int fchdir(int fd);

int creat(const char* path, bool_t directory);

int remove_file(const char* path);

int readdir(int fd, size_t offset, struct DirEntry* dirp);

int truncate(const char* path, size_t length);

int ftruncate(int fd, size_t length);

int waitpid(pid_t pid, int* wstatus);

#endif // YEHUDAOS_SYS
