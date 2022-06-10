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
/// It is safe to deinit after failed initialization
/// @return true on success, false on error
#define STACK_INIT(stack, initial_size) \
    ( \
        (stack).ptr = malloc_with_msg((initial_size) * sizeof((stack).ptr[0]), "Stack " #stack), \
        (stack).size = (initial_size), \
        (stack).used = 0, \
        !!((stack).ptr) \
    )

/// Free memory allocated by the stack
/// Idempotent
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

/// Return element of the stack at index i
#define STACK_AT(stack, i) \
    ((stack).ptr[i])

/// Return element of the stack at index i, counting from the back (0 is the last element)
#define STACK_AT_R(stack, i) \
    ((stack).ptr[(stack).used - 1 - (i)])
