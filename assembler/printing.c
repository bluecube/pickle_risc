#include "printing.h"

#include <stdarg.h>
#include <stdio.h>

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
