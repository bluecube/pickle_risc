#pragma once

#include "util.h"
#include <stdlib.h>
#include <string.h>

/// Calculating next size for stack inflations.
/// Increases the size by 50%
#define _STACK_GROWTH(old_size) ((old_size) + (old_size + 1) / 2)

inline size_t _stack_inflated_size(size_t oldSize, size_t sizeRequest) {
    size_t size = oldSize;
    while (size < sizeRequest)
        size = _STACK_GROWTH(size);
    return size;
}

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
#define _STACK_INFLATE(stack) \
    ( \
        (stack).size = _STACK_GROWTH((stack).size), \
        (stack).ptr = realloc_with_msg((stack).ptr, (stack).size * sizeof((stack).ptr[0]), "Stack " #stack), \
        !!((stack).ptr) \
    )

/// Push a value to the stack, inflate if necessary
/// @return true on success, false on error
#define STACK_PUSH(stack, value) \
    ( \
        (((stack).used < (stack).size) || _STACK_INFLATE(stack)) && \
        ((stack).ptr[(stack).used++] = value, true) \
    )

/// Make sure that the stack contains at least `newSize` allocated space.
/// Doesn't go through the exponential allocation
#define STACK_RESERVE(stack, newSize) \
    ( \
        (stack).size = ((stack).size < (newSize)) ? (newSize) : ((stack).size), \
        (stack).ptr = realloc_with_msg((stack).ptr, (stack).size * sizeof((stack).ptr[0]), "Stack " #stack), \
        !!((stack).ptr) \
    )

/// Change the used size of stack to newSize.
/// Fills new items with zeros.
#define STACK_RESIZE(stack, newSize) \
    ( \
        STACK_RESERVE(stack, _stack_inflated_size((stack).size, newSize)) && \
        ( \
            memset( \
                (stack).ptr + (stack).used, \
                0, \
                (newSize > (stack).size) ? (newSize - (stack).size) : 0 \
            ), \
            (stack).used = newSize, \
            1 \
        ) \
    )

/// Return element of the stack at index i
#define STACK_AT(stack, i) \
    ((stack).ptr[i])

/// Return element of the stack at index i, counting from the back (0 is the last element)
#define STACK_AT_R(stack, i) \
    ((stack).ptr[(stack).used - 1 - (i)])

typedef STACK_DECLARATION(char) char_stack_t;
