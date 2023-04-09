#define NULL (void*)0
#define TRUE 1
#define FALSE 0
#define FILE_NAME_LEN 11

typedef unsigned long size_t;
typedef unsigned char bool_t;

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

int read(int fd, void* buf, size_t count, size_t offset);

int write(int fd, const void* buf, size_t count, size_t offset);

int open(const char* pathname);

int fstat(int fd, struct Stat* statbuf);

void* malloc(size_t size);

void free(void* ptr);

int exec(const char* pathname);

void exit(int status);

int fchdir(int fd);

int creat(const char* path, bool_t directory);

int remove_file(const char* path);

int readdir(int fd, size_t offset, struct DirEntry* dirp);

int truncate(const char* path, size_t length);

int ftruncate(int fd, size_t length);
