#include "yehuda-os/helpers.h"
#include "yehuda-os/sys.h"

#define MAX_INT_STRLEN 11

const char* EXECUTABLE_PATH_START[] = { "./", "../", "/", NULL };

/**
 * Returns the amount of words in `str`.
 */
size_t count_words(const char* str)
{
    size_t count   = 0;
    bool_t in_word = FALSE;

    while (*str != '\0')
    {
        if (*str == ' ')
        {
            in_word = FALSE;
        }
        else if (!in_word)
        {
            in_word = TRUE;
            count++;
        }
        str++;
    }

    return count;
}

/**
 * Splits `command` into words separated by spaces.
 *
 * returns: An array of the words that are in the command,
 *          terminated by a NULL pointer or `NULL` on an allocation failure.
 *          All the elements in the array and the array itself must be freed by the user.
 */
char** parse_command(const char* command)
{
    const char* start   = NULL;
    size_t word_len     = 0;
    const char* current = command;
    char** words        = calloc(count_words(command) + 1, sizeof(char*));
    bool_t in_word      = FALSE;
    size_t count        = 0;

    if (words == NULL)
    {
        return NULL;
    }

    while (*current != '\0')
    {
        if (*current == ' ')
        {
            if (in_word)
            {
                words[count] = malloc((word_len + 1) * sizeof(char));
                if (words[count] == NULL)
                {
                    free_array((void**)words, count);
                    free(words);

                    return NULL;
                }
                strncpy(words[count], start, word_len);
                words[count][word_len] = '\0';

                count++;
                word_len = 0;
                in_word  = FALSE;
            }
        }
        else
        {
            if (!in_word)
            {
                in_word = TRUE;
                start   = current;
            }
            word_len++;
        }
        current++;
    }

    if (word_len > 0)
    {
        words[count] = malloc((word_len + 1) * sizeof(char));
        if (words[count] == NULL)
        {
            free_array((void**)words, count);
            free(words);

            return NULL;
        }
        strncpy(words[count], start, word_len);
        words[count][word_len] = '\0';
    }

    return words;
}

/**
 * Returns `TRUE` if a command is an executable file path or `FALSE` if it is a builtin.
 *
 * `command`: The command.
 */
bool_t is_executable(const char* command)
{
    bool_t executable            = FALSE;
    const char** current_str     = EXECUTABLE_PATH_START;
    const char* path_start_index = NULL;
    const char* command_index    = NULL;

    while (*current_str != NULL)
    {
        executable       = TRUE;
        path_start_index = *current_str;
        command_index    = command;
        while (*command_index != '\0' && executable)
        {
            if (*path_start_index == '\0')
            {
                return TRUE;
            }
            if (*path_start_index != *command_index)
            {
                executable = FALSE;
            }
            command_index++;
            path_start_index++;
        }
        current_str++;
    }

    return FALSE;
}

/**
 * Handles a builtin command.
 *
 * `argv`: The command that was entered, split into words.
 */
void handle_builtin(char* const argv[])
{
    if (strcmp(argv[0], "cd") == 0)
    {
        if (argv[1] == NULL)
        {
            print_str("YehudaSH: cd: No target parameter");
        }
        else if (chdir(argv[1]) == -1)
        {
            print_str("YehudaSH: cd: ");
            print_str(argv[1]);
            print_str(": No such file or directory");
        }
    }
    else
    {
        print_str("YehudaSH: ");
        print_str(argv[0]);
        print_str(": command not found");
    }
}

/**
 * Handles a command that executes a file.
 *
 * `argv`: The command that was entered, split into words.
 */
void handle_executable(char* const argv[])
{
    int exitcode                         = 0;
    pid_t pid                            = exec(argv[0], argv);
    char exitcode_buffer[MAX_INT_STRLEN] = { 0 };

    if (pid == -1)
    {
        print_str("YehudaSH: execution of ");
        print_str(argv[0]);
        print_str("has failed\n");

        return;
    }

    if (waitpid(pid, &exitcode) == -1)
    {
        print_str("Failed to retrieve the exit code of ");
        print_str(argv[0]);
    }
    else
    {
        int_to_string(exitcode, exitcode_buffer);
        print_str(argv[0]);
        print_str(" has exited with exit code ");
        print_str(exitcode_buffer);
    }
    print_str("\n");
}

/**
 * Gets a command from the user and handles it.
 *
 * returns: `TRUE` on success and `FALSE` on an allocation failure.
 *          Failures can occur when processing the command or reading it.
 */
bool_t handle_command()
{
    char* command       = NULL;
    char** command_args = NULL;
    char** current      = NULL;
    char* dir           = get_current_dir_name();

    if (dir == NULL)
    {
        return FALSE;
    }

    print_str("[YehudaSH] ");
    print_str(dir);
    print_str(" $ ");
    free(dir);
    dir = NULL;

    command = getline();
    if (command == NULL)
    {
        free(command);

        return FALSE;
    }
    else if ((command_args = parse_command(command)) == NULL)
    {
        return FALSE;
    }

    free(command);
    command = NULL;
    if (is_executable(command_args[0]))
    {
        handle_executable((char* const*)command_args);
    }
    else
    {
        handle_builtin((char* const*)command_args);
    }

    current = command_args;
    while (*current != NULL)
    {
        free(*current);
        *current = NULL;
        current++;
    }
    free(command_args);
    command_args = NULL;

    return TRUE;
}

int main()
{
    while (TRUE)
    {
        if (!handle_command())
        {
            print_str("YehudaSH: Allocating memory has failed.\n");
        }
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
