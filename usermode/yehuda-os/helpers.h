#ifndef YEHUDAOS_HELPERS
#define YEHUDAOS_HELPERS
#include "sys.h"

size_t strlen(const char* s);

char* strcpy(char* destination, const char* source);

char* strncpy(char* dest, const char* src, size_t n);

int strcmp(const char* str1, const char* str2);

int isspace(int c);

void free_array(void** arr, size_t size);

char* getline();

void print_str(const char* str);

void print_newline();

void int_to_string(int num, char* buffer);

#endif // YEHUDAOS_HELPERS
