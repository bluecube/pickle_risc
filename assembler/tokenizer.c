#include "tokenizer.h"
#include "util.h"

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
        fprintf(stderr, "Failed to allocate tokenizer buffer of size %zu\n", state->bufferSize);
        return false;
    }

    return true;
}

static void load_token(struct tokenizer_state* state) {
    int c = fgetc(state->fp);

    state->tokenBuffer.content = NULL;

    //printf("C = '%c' (0x%02x)\n", c, c);

    while (c == '#' || is_skippable_whitespace(c)) {
        if (c == '#') { // Skip over comments and whitespace
            do {
                c = fgetc(state->fp);
                //printf("skipping comment C = '%c' (0x%02x)\n", c, c);
            } while (c != EOF && c != '\n');
            continue;
        }

        while (is_skippable_whitespace(c)) { // Skip over whitespace
            c = fgetc(state->fp);
            //printf("skipping whitespace C = '%c' (0x%02x)\n", c, c);
        }
    }


    if (c == EOF) {
        if (feof(state->fp))
            state->tokenBuffer.type = TOKEN_EOF;
        else {
            fprintf(stderr, "Error reading file\n");
            state->tokenBuffer.type = TOKEN_ERROR;
        }
    }
    else if (c == '\n' || c == ';')
        state->tokenBuffer.type = TOKEN_EOL;
    else if (c == ':' || c == ',')
        state->tokenBuffer.type = c;
    else if (is_identifier_first_char(c)) {
        size_t length = 0;
        do {
            if (length >= state->bufferSize)
                inflate_buffer(state);
            state->buffer[length++] = c;
            c = fgetc(state->fp);
        } while (is_identifier_char(c));
        if (c != EOF)
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
        state->tokenBuffer.contentLength = length;
    }
    else
        state->tokenBuffer.type = TOKEN_ERROR;
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
    // Clear the state, so that a tokenizer that failed to open can still be safely passed
    // to close and it is a no-op.
    state->buffer = NULL;
    state->tokenBuffer.type = TOKEN_NONE;
    state->tokenBuffer.content = NULL;

    state->fp = fopen(filename, "rb");
    if (!state->fp) {
        fprintf(stderr, "Failed to open file `%s`\n", filename);
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
