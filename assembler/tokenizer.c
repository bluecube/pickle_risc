#include "tokenizer.h"
#include "util.h"
#include "printing.h"

#include <ctype.h>
#include <assert.h>
#include <string.h>

static bool is_identifier_first_char(int c) {
    return c == '.' || c == '_' || c == '?' || isalpha(c);
}

static bool is_identifier_char(int c) {
    return is_identifier_first_char(c) || isdigit(c);
}

static bool is_skippable_whitespace(int c) {
    return c != '\n' && isspace(c);
}

static bool inflate_buffer(struct tokenizer_state* state) {
    state->bufferSize *= 2;
    state->buffer = realloc(state->buffer, state->bufferSize);
    if (!state->buffer) {
        error("Allocating %zuB for tokenizer buffer failed\n", state->bufferSize);
        return false;
    }

    return true;
}

static int tok_getc(struct tokenizer_state* state) {
    int c = fgetc(state->fp);
    if (c == '\n') {
        state->location.line += 1;
        state->location.column = 0;
    } else
        state->location.column += 1;

    return c;
}

static void unexpected_character_error(struct location location, int c) {
    int printC = c;
    if (!isgraph(c))
        c = ' ';
    localized_error(location, "Unexpected character '%c'(0x%02x)", printC, c);
}

/// Parse a single positive number from tokenizer, starting with a character c.
/// @return number or negative on error.
static numeric_value_t parse_number(struct tokenizer_state* state, int c) {
    int base;
    numeric_value_t ret = 0;
    bool haveDigits = false;

    if (c == '0') {
        // next character determines the base
        c = tok_getc(state);
        switch (c) {
        case 'x':
        case 'X':
            base = 16;
            break;
        case 'o':
        case 'O':
            base = 8;
            break;
        case 'b':
        case 'B':
            base = 2;
            break;
        default:
            unexpected_character_error(state->location, c);
            return -1;
        }
    } else {
        base = 10;
        ret = parse_digit(c);
        assert(ret > 0);
        assert(ret < 10);
        haveDigits = true;
    }

    while (true) {
        struct location locationBackup = state->location;
        c = tok_getc(state);
        int d = parse_digit(c);
        if (d < 0 || d >= base) {
            if (haveDigits) {
                ungetc(c, state->fp);
                state->location = locationBackup;
                return ret;
            } else {
                localized_error(state->tokenBuffer.location, "Base-%d numeric literal with no digits", base);
                return -1;
            }
        }

        bool overflow = __builtin_mul_overflow(ret, base, &ret);
        overflow = overflow || __builtin_add_overflow(ret, d, &ret);

        if (overflow) {
            localized_error(state->tokenBuffer.location, "Numeric literal overflow");
            return -1;
        }
        haveDigits = true;
    }
}

static void load_token(struct tokenizer_state* state) {
    int c = tok_getc(state);

    state->tokenBuffer.content = NULL;

    while (c == '#' || is_skippable_whitespace(c)) {
        if (c == '#') { // Skip over comments and whitespace
            do {
                c = tok_getc(state);
            } while (c != EOF && c != '\n');
            continue;
        }

        while (is_skippable_whitespace(c)) { // Skip over whitespace
            c = tok_getc(state);
        }
    }

    state->tokenBuffer.location = state->location;

    if (c == EOF) {
        if (feof(state->fp))
            state->tokenBuffer.type = TOKEN_EOF;
        else {
            error("Error reading file");
            state->tokenBuffer.type = TOKEN_ERROR;
        }
    }
    else if (c == '\n' || c == ';')
        state->tokenBuffer.type = TOKEN_EOL;
    else if (c == ':' || c == ',')
        state->tokenBuffer.type = c;
    else if (is_identifier_first_char(c)) {
        size_t length = 0;
        struct location locationBackup;
        do {
            if (length >= state->bufferSize)
                inflate_buffer(state);
            state->buffer[length++] = c;
            locationBackup = state->location;
            c = tok_getc(state);
        } while (is_identifier_char(c));

        state->location = locationBackup;
        ungetc(c, state->fp);

        char* identifierCopy = malloc_with_msg(length + 1, "token");
        if (!identifierCopy) {
            state->tokenBuffer.type = TOKEN_ERROR;
            return;
        }
        memcpy(identifierCopy, state->buffer, length);
        identifierCopy[length] = '\0';

        state->tokenBuffer.type = TOKEN_IDENTIFIER;
        state->tokenBuffer.content = identifierCopy;
        state->tokenBuffer.contentNumeric = length;
    }
    else if (c >= '0' && c <= '9') {
        numeric_value_t value = parse_number(state, c);
        if (value < 0) {
            state->tokenBuffer.type = TOKEN_ERROR;
        } else {
            state->tokenBuffer.type = TOKEN_NUMBER;
            state->tokenBuffer.contentNumeric = value;
        }
    }
    else {
        unexpected_character_error(state->location, c);
        state->tokenBuffer.type = TOKEN_ERROR;
    }
}

struct token get_token(struct tokenizer_state* state) {
    if (state->tokenBuffer.type == TOKEN_NONE)
        load_token(state);

    struct token ret = state->tokenBuffer;
    state->tokenBuffer.type = TOKEN_NONE;
    state->tokenBuffer.content = NULL;

    return ret;
}

void unget_token(struct token token, struct tokenizer_state* state) {
    assert(state->tokenBuffer.type == TOKEN_NONE);
    state->tokenBuffer = token;
}

bool tokenizer_open(const char* filename, struct tokenizer_state* state) {
    state->location.filename = filename;
    state->location.line = 1;
    state->location.column = 0;

    // Clear the state, so that a tokenizer that failed to open can still be safely passed
    // to close and it is a no-op.
    state->buffer = NULL;
    state->tokenBuffer.type = TOKEN_NONE;
    state->tokenBuffer.content = NULL;

    state->fp = fopen(filename, "rb");
    if (!state->fp) {
        error("%s: Failed to open file", filename);
        return false;
    }

    state->bufferSize = 128 / 2; // Size will be doubled by inflate_buffer.
    if (!inflate_buffer(state)) {
        fclose(state->fp);
        state->fp = NULL;
        return false;
    }

    return true;
}

void tokenizer_close(struct tokenizer_state* state) {
    if (state->fp)
        fclose(state->fp);
    state->fp = NULL;

    if (state->buffer)
        free(state->buffer);
    state->buffer = NULL;

    free_token(&(state->tokenBuffer));
}

void free_token(struct token *token) {
    if (token->content)
        free(token->content);
    token->content = NULL;
    token->type = TOKEN_NONE;
}

char* free_token_move_content(struct token *token) {
    char* ret = token->content;
    token->content = NULL;
    token->type = TOKEN_NONE;
    return ret;
}
