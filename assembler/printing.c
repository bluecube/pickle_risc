#include "printing.h"

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <assert.h>
#include <string.h>

int error(const char* format, ...) {
    int ret1 = fprintf(stderr, "error: ");
    if (ret1 < 0)
        return ret1;

    va_list ap;
    va_start(ap, format);
    int ret2 = vfprintf(stderr, format, ap);
    if (ret2 < 0)
        return ret2;
    va_end(ap);

    int ret3 = fprintf(stderr, "\n");
    if (ret3 < 0)
        return ret3;

    return ret1 + ret2 + ret3;
}

int localized_error(struct location location, const char* format, ...) {
    int ret1 = fprintf(stderr, "%s:%u:%u: error: ", location.filename, location.line, location.column);
    if (ret1 < 0)
        return ret1;

    va_list ap;
    va_start(ap, format);
    int ret2 = vfprintf(stderr, format, ap);
    if (ret2 < 0)
        return ret2;
    va_end(ap);

    int ret3 = fprintf(stderr, "\n");
    if (ret3 < 0)
        return ret3;

    return ret1 + ret2 + ret3;
}

bool push_identifier_to_buffer(struct token *identifier, print_buffer_t *buffer) {
    assert(identifier->type == TOKEN_IDENTIFIER);

    if (!buffer || !buffer->ptr)
        return true;

    size_t newSize = buffer->used + identifier->contentNumeric;
    if (!STACK_RESERVE(*buffer, newSize))
        return false;

    memcpy(
        &STACK_AT_R(*buffer, -1),
        identifier->content,
        identifier->contentNumeric
    );
    buffer->used = newSize;

    return true;

}

bool printf_to_buffer(print_buffer_t *buffer, const char* format, ...) {
    if (!buffer || !buffer->ptr)
        return true;

    size_t availableSize = buffer->size - buffer->used;

    va_list ap;

    va_start(ap, format);
    int printedLength = vsnprintf(&STACK_AT_R(*buffer, -1), availableSize, format, ap);
    va_end(ap);

    if (printedLength < 0) {
        error("Printf to buffer failed");
        return false;
    }

    if ((size_t)printedLength <= availableSize) {
        buffer->used += printedLength; // This does not include the terminating '\0'
        return true;
    }

    if (!STACK_RESERVE(*buffer, buffer->used + printedLength))
        return false;

    va_start(ap, format);
    printedLength = vsnprintf(&STACK_AT_R(*buffer, -1), availableSize, format, ap);
    va_end(ap);

    if (printedLength < 0) {
        error("Printf to buffer failed");
        return false;
    }

    assert(buffer->used + (size_t)printedLength <= buffer->size);

    buffer->used += printedLength; // This does not include the terminating '\0'

    return true;
}

