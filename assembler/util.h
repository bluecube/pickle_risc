#pragma once

#include <stdlib.h>

void* malloc_with_msg(size_t size, const char* label);

/// Converts a character digit to its value, or returns -1 if it is not a digit.
/// Supports hexadecimal lowercase or uppercase digits.
int parse_digit(int c);
