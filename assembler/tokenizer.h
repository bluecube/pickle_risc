#pragma once
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>

#define TOKEN_NONE -1
#define TOKEN_ERROR -2
#define TOKEN_EOF -3
#define TOKEN_EOL -4
#define TOKEN_IDENTIFIER -5
/*#define TOKEN_NUMBER -6
#define TOKEN_QUOTED_STRING -7*/
// Single characters tokens are represented by the character itself.

struct token {
    char* content;
    size_t contentLength;
    int type;
};

struct tokenizer_state {
    FILE* fp;
    char* buffer;
    size_t bufferSize;
    struct token tokenBuffer;
};

bool tokenizer_open(const char* filename, struct tokenizer_state* state);

/// Close tokenizer, free all resources. Idempotent.
void tokenizer_close(struct tokenizer_state* state);
struct token get_token(struct tokenizer_state* state);
void unget_token(struct token token, struct tokenizer_state* state);
void free_token(struct token *token);
char* free_token_move_content(struct token* token);
