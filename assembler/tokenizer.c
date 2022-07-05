#include "tokenizer.h"
#include "util.h"
#include "printing.h"

#include <ctype.h>
#include <assert.h>
#include <string.h>

static bool is_identifier_first_char(int c) {
    return c == '.' || c == '_' || isalpha(c);
}

static bool is_identifier_char(int c) {
    return is_identifier_first_char(c) || isdigit(c) || c == '?';
}

static bool is_skippable_whitespace(int c) {
    return c != '\n' && isspace(c);
}

static bool is_simple_token(int c) {
    return !!strchr(":,(){}+-/%~", c);
}

/// Parse multi character tokens in format `cc` and `c=` (were `c` is the value of the parameter).
/// @return token type
static int parse_magic_token(struct tokenizer_state* state, int c, int doubleCharTokenType, int eqCharTokenType) {
    int c2;
    if (!localized_file_getc(&state->f, &c2))
        return TOKEN_ERROR;

    if (c2 == c && doubleCharTokenType != TOKEN_NONE)
        return doubleCharTokenType;
    else if (c2 == '=' && eqCharTokenType != TOKEN_NONE)
        return eqCharTokenType;
    else {
        localized_file_ungetc(&state->f, c2);
        return c;
    }
}

static int parse_string_literal_escape(struct tokenizer_state* state) {
    int c;
    if (!localized_file_getc(&state->f, &c))
        return -1;
    switch (c) {
    case TOKEN_ERROR:
        return -1;
    case 'a':
        return '\a';
    case 'b':
        return '\b';
    case 'f':
        return '\f';
    case 'n':
        return '\n';
    case 'r':
        return '\r';
    case 't':
        return '\t';
    case 'v':
        return '\v';
    case '0':
        return '\0';
    case '\'':
    case '"':
    case '?':
    case '\\':
        return c;
    case 'x': {
            int c1;
            if (!localized_file_getc(&state->f, &c1))
                return -1;
            int d1 = parse_digit(c1);
            if (d1 < 0) {
                localized_error(state->f.location, "Invalid escape sequence: Expected hex digit");
                return -1;
            }
            int c2;
            if (!localized_file_getc(&state->f, &c2))
                return -1;
            int d2 = parse_digit(c2);
            if (d2 < 0) {
                localized_error(state->f.location, "Invalid escape sequence: Expected hex digit");
                return -1;
            }
            return d1 << 4 | d2;
        }
    default:
        localized_error(state->f.location, "Invalid escape sequence");
        return -1;
    }
}

static char *parse_string(struct tokenizer_state* state, numeric_value_t* length) {
    struct location startLocation = state->f.location;

    while (true) {
        int c;
        if (!localized_file_getc(&state->f, &c))
            return NULL;

        if (c == EOF || c == '\n') {
            localized_error(state->f.location, "Unexpected end of string");
            return NULL;
        } else if (c == '"')
            break;
        if (c == '\\') {
            int c2 = parse_string_literal_escape(state);
            if (c2 < 0)
                return NULL;

            if (!STACK_PUSH(state->buffer, c2))
                return NULL;

        } else {
            if (!STACK_PUSH(state->buffer, c))
                return NULL;
        }
    }

    if (!STACK_PUSH(state->buffer, '\0'))
        return NULL;

    *length = state->buffer.used - 1;
    if (*length < 0 || (size_t)(*length) != state->buffer.used - 1) {
        localized_error(startLocation, "Too long string");
        return NULL;
    }

    return state->buffer.ptr;
}

/// Parse an identifier from tokenizer, starting with character c.
static char* parse_identifier(struct tokenizer_state* state, int c, numeric_value_t* length) {
    struct location startLocation = state->f.location;
    do {
        if (!STACK_PUSH(state->buffer, c))
            return NULL;
        if (!localized_file_getc(&state->f, &c))
            return NULL;
    } while (is_identifier_char(c));

    localized_file_ungetc(&state->f, c);

    if (!STACK_PUSH(state->buffer, '\0'))
        return NULL;

    *length = state->buffer.used - 1;
    if (*length < 0 || (size_t)(*length) != state->buffer.used - 1) {
        localized_error(startLocation, "Too long identifier");
        return NULL;
    }

    return state->buffer.ptr;
}

/// Parse a single positive number from tokenizer, starting with a character c.
/// @return number or negative on error.
static numeric_value_t parse_number(struct tokenizer_state* state, int c) {
    int base;
    numeric_value_t ret = 0;
    bool haveDigits = false;

    struct location startLocation = state->f.location;

    if (c == '0') {
        // next character determines the base, or zero
        if (!localized_file_getc(&state->f, &c))
            return -1;

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
            if (isdigit(c)) {
                localized_error(state->f.location, "Leading zero in decimal integer literal");
                return -1;
            } else {
                // Decimal zero
                localized_file_ungetc(&state->f, c);
                return 0;
            }
        }
    } else {
        base = 10;
        ret = parse_digit(c);
        assert(ret > 0);
        assert(ret < 10);
        haveDigits = true;
    }

    while (true) {
        if (!localized_file_getc(&state->f, &c))
            return -1;
        if (c == '_')
            continue;
        int d = parse_digit(c);
        if (d < 0 || d >= base) {
            if (haveDigits) {
                localized_file_ungetc(&state->f, c);
                return ret;
            } else {
                localized_error(startLocation, "Base-%d numeric literal with no digits", base);
                return -1;
            }
        }

        bool overflow = __builtin_mul_overflow(ret, base, &ret);
        overflow = overflow || __builtin_add_overflow(ret, d, &ret);

        if (overflow) {
            localized_error(startLocation, "Numeric literal overflow");
            return -1;
        }
        haveDigits = true;
    }
}

/// Load a token into the peek buffer
static void load_next_token(struct tokenizer_state* state) {
    state->peekBuffer.type = TOKEN_ERROR;

    int c;
    if (!localized_file_getc(&state->f, &c))
        return;

    while (c == '#' || is_skippable_whitespace(c)) {
        if (c == '#') { // Skip over comments and whitespace
            do {
                if (!localized_file_getc(&state->f, &c))
                    return;
            } while (c != EOF && c != '\n');
            continue;
        }

        while (is_skippable_whitespace(c)) { // Skip over whitespace
            if (!localized_file_getc(&state->f, &c))
                return;
        }
    }

    state->peekBuffer.location = state->f.location;
    state->peekBuffer.content = NULL;
    state->buffer.used = 0;

    if (c == EOF)
        state->peekBuffer.type = TOKEN_EOF;
    else if (c == '\n' || c == ';')
        state->peekBuffer.type = TOKEN_EOL;
    else if (is_simple_token(c))
        state->peekBuffer.type = c;
    else if (c == '!') // "!="
        state->peekBuffer.type = parse_magic_token(state, c, TOKEN_NONE, TOKEN_OPERATOR_NEQ);
    else if (c == '<') // "<<" "<="
        state->peekBuffer.type = parse_magic_token(state, c, TOKEN_OPERATOR_SHL, TOKEN_OPERATOR_LE);
    else if (c == '>') // ">>" ">="
        state->peekBuffer.type = parse_magic_token(state, c, TOKEN_OPERATOR_SHR, TOKEN_OPERATOR_GE);
    else if (c == '*') // "**"
        state->peekBuffer.type = parse_magic_token(state, c, TOKEN_OPERATOR_POWER, TOKEN_NONE);
    else if (c == '&') // "&&"
        state->peekBuffer.type = parse_magic_token(state, c, TOKEN_OPERATOR_LOGICAL_AND, TOKEN_NONE);
    else if (c == '|') // "||"
        state->peekBuffer.type = parse_magic_token(state, c, TOKEN_OPERATOR_LOGICAL_OR, TOKEN_NONE);
    else if (c == '=') // "=="
        state->peekBuffer.type = parse_magic_token(state, c, TOKEN_OPERATOR_EQ, TOKEN_NONE);
    else if (c == '"') {
        state->peekBuffer.content = parse_string(state, &state->peekBuffer.contentNumeric);
        if (state->peekBuffer.content)
            state->peekBuffer.type = TOKEN_STRING;
    } else if (is_identifier_first_char(c)) {
        state->peekBuffer.content = parse_identifier(state, c, &state->peekBuffer.contentNumeric);
        if (state->peekBuffer.content)
            state->peekBuffer.type = TOKEN_IDENTIFIER;
    } else if (isdigit(c)) {
        state->peekBuffer.contentNumeric = parse_number(state, c);
        if (state->peekBuffer.contentNumeric >= 0)
            state->peekBuffer.type = TOKEN_NUMBER;
    } else
        localized_error(state->f.location, "Unexpected character");
}

struct token get_token(struct tokenizer_state* state) {
    struct token ret = state->peekBuffer;

    if (ret.content) {
        assert(ret.content == state->buffer.ptr);
        size_t copiedSize = ret.contentNumeric + 1; // add byte for termination
        ret.content = malloc_with_msg(copiedSize, "get_token content copy");
        if (!ret.content) {
            ret.type = TOKEN_ERROR;
            return ret;
        }
        memcpy(ret.content, state->peekBuffer.content, copiedSize);
    }

    load_next_token(state);

    return ret;
}

struct token *peek_token(struct tokenizer_state *state) {
    return &(state->peekBuffer);
}

bool skip_token(struct tokenizer_state *state) {
    bool ret = (state->peekBuffer.type != TOKEN_ERROR);
    load_next_token(state);
    return ret;
}

bool tokenizer_open(const char* filename, struct tokenizer_state* state) {
    // Clear the state, so that a tokenizer that failed to open can still be safely passed
    // to close and it is a no-op.
    state->buffer.ptr = NULL;
    state->peekBuffer.type = TOKEN_ERROR;
    state->peekBuffer.content = NULL;

    if (!localized_file_open(&state->f, filename))
        return false;

    if (!STACK_INIT(state->buffer, 32)) {
        localized_file_close(&state->f);
        return false;
    }

    load_next_token(state);

    return true;
}

void tokenizer_close(struct tokenizer_state* state) {
    localized_file_close(&state->f);
    STACK_DEINIT(state->buffer);
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

const char* readable_token_type(int tokenType) {
    static char retBuffer[2] = {'\0', '\0'};
    switch (tokenType) {
    case TOKEN_NONE:
        return "none";
    case TOKEN_ERROR:
        return "error";
    case TOKEN_EOF:
        return "eof";
    case TOKEN_EOL:
        return "eol";
    case TOKEN_IDENTIFIER:
        return "identifier";
    case TOKEN_NUMBER:
        return "number";
    case TOKEN_OPERATOR_EQ:
        return "==";
    case TOKEN_OPERATOR_NEQ:
        return "!=";
    case TOKEN_OPERATOR_LE:
        return "<=";
    case TOKEN_OPERATOR_GE:
        return ">=";
    case TOKEN_OPERATOR_SHL:
        return "<<";
    case TOKEN_OPERATOR_SHR:
        return ">>";
    case TOKEN_OPERATOR_POWER:
        return "**";
    case TOKEN_OPERATOR_LOGICAL_AND:
        return "&&";
    case TOKEN_OPERATOR_LOGICAL_OR:
        return "||";
    case TOKEN_STRING:
        return "string";
    default:
        if (tokenType > 0) {
            retBuffer[0] = tokenType;
            return retBuffer;
        } else
            return "!!!!!!!!";
    }
}
