#pragma once
#include <stdlib.h>

#define ARRAY_SIZE(a) (sizeof(a) / sizeof(a[0]))

void* malloc_with_msg(size_t size, const char* label);
void* realloc_with_msg(void* ptr, size_t size, const char* label);
char* strdup_with_msg(const char* s, const char* label);

/// Converts a character digit to its value, or returns -1 if it is not a digit.
/// Supports hexadecimal lowercase or uppercase digits.
int parse_digit(int c);
