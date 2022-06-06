#pragma once
#include "tokenizer.h"

/// Print an error message to stderr.
/// Adds a newline
int error(const char* format, ...)
    __attribute__ ((format (printf, 1, 2)));

/// Print an error message to stderr, including location.
/// Adds a newline
int localized_error(struct location location, const char* format, ...)
    __attribute__ ((format (printf, 2, 3)));
