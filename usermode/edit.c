#include "yehuda-os/helpers.h"
#include "yehuda-os/sys.h"

int main(int argc, char* argv[])
{
    int fd = 0;

    if (argc <= 1)
    {
        print_str("edit: missing file operand\n"
                  "Usage: edit <file>\n");

        return 1;
    }

    fd = open(argv[1]);
    if (fd == -1)
    {
        print_str("edit: file does not exist.\n");

        return 1;
    }

    // clear the file memory
    struct Stat stat = {.directory = 0, .size = 0};
    fstat(fd, &stat);

    if (stat.directory == TRUE)
    {
        print_str("cant edit a folder");
        print_newline();
        return 1;
    }

    char *empty = (char *)calloc(stat.size, sizeof(char));
    write(fd, (void *)empty, stat.size, 0);

    char *curr_line = NULL;
    char content[1024] = "";

    while (1)
    {
        curr_line = getline();

        if (strlen(curr_line) == 0 || curr_line == NULL)
        {
            break;
        }
        strcat(content, curr_line);
        strcat(content, " \n");
        free(curr_line);
        curr_line = NULL;
    }

    write(fd, content, strlen(content), 0);
    free(curr_line);
    free(empty);

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
