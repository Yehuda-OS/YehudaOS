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

int main(int argc, char **argv)
{
    if (argc <= 1)
    {
        print_str("touch: missing file operand\n"
                  "Usage: touch <file>\n");

        return 1;
    }
    char *slash = NULL;
    if ((slash = strrchr(argv[1], '/')) != NULL)
    {
        struct Stat stat = {.directory = 0, .size = 0};
        char *path = malloc((slash - argv[1]) * sizeof(char));
        size_t len = (slash - argv[1]) + 1; // Calculate the length of the substring
        strncpy(path, argv[1], len);        // Copy the substring to dest
        path[len] = '\0';                   // Null-terminate dest

        print_str(path);
        print_newline();
        int fd = open(path);
        if (fd != -1)
        {
            fstat(fd, &stat);
            if (stat.directory == FALSE)
            {
                print_str("path is a file and not a folder");
                print_newline();
                return 1;
            }
        }
        free(path);
    }
    if (creat(argv[1], FALSE) == -1)
    {
        print_str("touch: failed to create file\n");

        return 1;
    }

    return 0;
}
