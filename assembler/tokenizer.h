#pragma once
#include "stack.h"

#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <inttypes.h>

#define TOKEN_NONE -1
#define TOKEN_ERROR -2
#define TOKEN_EOF -3
#define TOKEN_EOL -4
#define TOKEN_IDENTIFIER -5
#define TOKEN_NUMBER -6
//#define TOKEN_QUOTED_STRING -7
// Single characters tokens are represented by the character itself.

typedef int32_t numeric_value_t;
#define NUMERIC_VALUE_FORMAT PRId32

struct location {
    const char* filename;
    unsigned line;
    unsigned column;
};

struct token {
    char* content;
    numeric_value_t contentNumeric;
    int type;

    struct location location;
};

struct tokenizer_state {
    FILE* fp;
    STACK_DECLARATION(char) buffer;
    struct token tokenBuffer;

    struct location location;
};

/// Open given file for tokenization.
/// filename must remain valid until state is closed.
/// @return true if successful, otherwise prints an error and exits.
bool tokenizer_open(const char* filename, struct tokenizer_state* state);

/// Close tokenizer, free all resources. Idempotent.
void tokenizer_close(struct tokenizer_state* state);
struct token get_token(struct tokenizer_state* state);
void unget_token(struct token token, struct tokenizer_state* state);
void free_token(struct token *token);
char* free_token_move_content(struct token* token);
