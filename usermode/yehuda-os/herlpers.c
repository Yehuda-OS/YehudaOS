#include "helpers.h"

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

    while (*source)
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
