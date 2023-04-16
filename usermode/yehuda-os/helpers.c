#include "helpers.h"
#include "sys.h"

size_t strlen(const char* s)
{
    size_t count = 0;

    while (*s)
    {
        count++;
        s++;
    }

    return count;
}

char* strcpy(char* destination, const char* source)
{
    char* ptr = destination;

    if (destination == NULL)
    {
        return NULL;
    }

    while (*source != '\0')
    {
        *destination = *source;
        destination++;
        source++;
    }
    *destination = '\0';

    return ptr;
}

char* strncpy(char* dest, const char* src, size_t n)
{
    size_t i;

    for (i = 0; i < n && src[i] != '\0'; i++)
    {
        dest[i] = src[i];
    }
    for (; i < n; i++)
    {
        dest[i] = '\0';
    }

    return dest;
}

/**
 * Free all the elements of an array of pointers `arr` with length of `size`.
 */
void free_array(void** arr, size_t size)
{
    for (size_t i = 0; i < size; i++)
    {
        free(arr[i]);
        arr[i] = NULL;
    }
}

/**
 * Reads a line from the console.
 *
 * returns: The line that was read or `NULL` on an allocation failure.
 *          The returned buffer must be freed by the caller.
 */
char* getline()
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

            if (buffer == NULL)
            {
                return NULL;
            }
        }

        bytes_read = read(STDIN, buffer + current, 1, 0);
        if (bytes_read == -1)
        {
            free(buffer);

            return NULL;
        }
        else
        {
            current += bytes_read;
        }
    } while (buffer[current - bytes_read] != '\n');
    buffer[current - bytes_read] = '\0';

    return buffer;
}

/**
 * Print a string `str` to the screen.
 */
void print_str(const char* str)
{
    write(STDOUT, str, strlen(str), 0);
}

/**
 * Convert an integer to a string.
 *
 * `num`: The number to convert.
 * `buffer`: The string to put the result into.
 *           Must be at least 11 bytes long.
 */
void int_to_string(int num, char* buffer)
{
    int i        = 0;
    int num_copy = 0;

    if (num == 0)
    {
        buffer[0] = '0';
        buffer[1] = '\0';

        return;
    }

    if (num < 0)
    {
        buffer[0] = '-';
        num       = -num;
        i         = 1;
    }

    num_copy = num;
    while (num_copy > 0)
    {
        num_copy /= 10;
        i++;
    }

    buffer[i] = '\0';
    while (num > 0)
    {
        i--;
        buffer[i] = '0' + (num % 10);
        num /= 10;
    }
}
