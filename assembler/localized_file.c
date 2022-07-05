#include "localized_file.h"
#include "printing.h"

bool localized_file_open(struct localized_file *this, const char *filename) {
    this->ungetcChar = -1;

    this->location.filename = filename;
    this->location.line = 1;
    this->location.column = 0;

    this->fp = fopen(filename, "rb");
    if (!this->fp) {
        error("%s: Failed to open file", filename);
        return false;
    } else
        return true;
}

void localized_file_close(struct localized_file *this) {
    if (this->fp)
        fclose(this->fp);
}

bool localized_file_getc(struct localized_file *this, int *c) {
    if (this->ungetcChar > 0) {
        *c = this->ungetcChar;
        this->ungetcChar = -1;
        return true;
    }

    *c = fgetc(this->fp);
    switch (*c) {
    case EOF:
        if (!feof(this->fp)) {
            localized_error(this->location, "Error reading file");
            return false;
        }
        break;
    case '\n':
        this->location.line += 1;
        this->location.column = 0;
        break;
    default:
        this->location.column += 1;
        break;
    }

    return true;
}

void localized_file_ungetc(struct localized_file *this, int c) {
    this->ungetcChar = c;
}
