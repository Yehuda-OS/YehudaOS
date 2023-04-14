#include "yehuda-os.h"

const char* EXECUTABLE_PATH_START[] = { "./", "../", "/" };

/**
 * Reads a line from the console and returns it.
 */
char* get_command()
{
    ssize_t bytes_read = 0;
    size_t current     = 0;
    size_t len         = 1;
    char* buffer       = NULL;

    do
    {
        if (current == len - 1)
        {
            len *= 2;
            buffer = realloc(buffer, len);

            if (!buffer)
            {
                return NULL;
            }
        }

        if ((bytes_read = read(stdin, buffer + current, 1, 0)) == -1)
        {
            free(buffer);

            return NULL;
        }
        else
        {
            current += bytes_read;
        }
    } while (buffer[current] != '\n');
    buffer[current] = 0;

    return buffer;
}

/**
 * Splits `command` into words separated by spaces and returns an array of them.
 */
char** parse_command(char* command)
{
    return NULL;
}

/**
 * Handles a builtin command.
 */
void handle_builtin()
{
}

/**
 * Handles a command that executes a file.
 */
void handle_executable()
{
}

int main()
{
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
    // tell the compiler to make sure side effects are done before the asm statement
    __builtin_unreachable();
}
