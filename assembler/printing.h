#pragma once
#include "tokenizer.h"
#include "stack.h"

/// Print an error message to stderr.
/// Adds a newline
int error(const char* format, ...)
    __attribute__ ((format (printf, 1, 2)));

/// Print an error message to stderr, including location.
/// Adds a newline
int localized_error(struct location location, const char* format, ...)
    __attribute__ ((format (printf, 2, 3)));


typedef STACK_DECLARATION(char) print_buffer_t;

bool push_identifier_to_buffer(struct token *identifier, print_buffer_t *buffer);
bool printf_to_buffer(print_buffer_t *buffer, const char* format, ...)
    __attribute__ ((format (printf, 2, 3)));
