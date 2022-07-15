#pragma once
#include <stdlib.h>
#include <stdbool.h>

#define ARRAY_SIZE(a) (sizeof(a) / sizeof(a[0]))

void* malloc_with_msg(size_t size, const char* label);
void* realloc_with_msg(void* ptr, size_t size, const char* label);
char* strdup_with_msg(const char* s, const char* label);

/// Converts a character digit to its value, or returns -1 if it is not a digit.
/// Supports hexadecimal lowercase or uppercase digits.
int parse_digit(int c);

/// Return true if two semi-open intervals overlap
bool intervals_overlap(size_t start1, size_t end1, size_t start2, size_t end2);
