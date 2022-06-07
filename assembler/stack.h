#pragma once

#include "util.h"
#include <stdlib.h>

#define STACK_DECLARATION(type) \
    struct { \
        type* ptr; \
        size_t size; \
        size_t used; \
    }

/// Initialize the stack with given initial size.
/// @return true on success, false on error
#define STACK_INIT(stack, initial_size) \
    ( \
        (stack).ptr = malloc_with_msg((initial_size) * sizeof((stack).ptr[0]), "Stack " #stack), \
        (stack).size = (initial_size), \
        (stack).used = 0, \
        !!((stack).ptr) \
    )

/// Free memory allocated by the stack
#define STACK_DEINIT(stack) \
    do { \
        free((stack).ptr); \
        (stack).ptr = NULL; \
        (stack).size = (stack).used = 0; \
    } while(0)

/// Double the allocated size of the stack.
/// @return true on success, false on error
#define STACK_INFLATE(stack) \
    ( \
        (stack).size *= 2, \
        (stack).ptr = realloc_with_msg((stack).ptr, (stack).size * sizeof((stack).ptr[0]), "Stack " #stack), \
        !!((stack).ptr) \
    )

/// Push a value to the stack, inflate if necessary
/// @return true on success, false on error
#define STACK_PUSH(stack, value) \
    ( \
        (((stack).used < (stack).size) || STACK_INFLATE(stack)) && \
        ((stack).ptr[(stack).used++] = value, true) \
    )
