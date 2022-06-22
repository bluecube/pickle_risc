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

static int tok_getc(struct tokenizer_state* state) {
    int c = fgetc(state->fp);
    if (c == EOF) {
        if (feof(state->fp))
            return TOKEN_EOF;
        else
            return TOKEN_ERROR;
    } else if (c == '\n') {
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

/// Parse tokens that have different meaning when doubled (eg. <<)
/// @return token type
static int parse_magic_token(struct tokenizer_state* state, int c, int doubleCharTokenType, int eqCharTokenType) {
    struct location locationBackup = state->location;
    int c2 = tok_getc(state);
    if (c2 == TOKEN_ERROR)
        return TOKEN_ERROR;
    else if (c2 == c && doubleCharTokenType != TOKEN_NONE)
        return doubleCharTokenType;
    else if (c2 == '=' && eqCharTokenType != TOKEN_NONE)
        return eqCharTokenType;
    else {
        state->location = locationBackup;
        ungetc(c2, state->fp);
        return c;
    }
}

static int parse_string_literal_escape(struct tokenizer_state* state) {
    int c = tok_getc(state);
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
            int c1 = tok_getc(state);
            if (c1 == TOKEN_ERROR)
                return -1;
            int d1 = parse_digit(c1);
            if (d1 < 0) {
                localized_error(state->location, "Invalid escape sequence: Expected hex digit");
                return -1;
            }
            int c2 = tok_getc(state);
            if (c2 == TOKEN_ERROR)
                return -1;
            int d2 = parse_digit(c2);
            if (d2 < 0) {
                localized_error(state->location, "Invalid escape sequence: Expected hex digit");
                return -1;
            }
            return d1 << 4 | d2;
        }
    default:
        localized_error(state->location, "Invalid escape sequence");
        return -1;
    }
}

static char *parse_string(struct tokenizer_state* state, numeric_value_t* length) {
    while (true) {
        int c = tok_getc(state);

        if (c == TOKEN_ERROR) {
            return NULL;
        } else if (c == TOKEN_EOF || c == '\n') {
            localized_error(state->location, "Unexpected end of string");
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
        localized_error(state->location, "Too long string");
        return NULL;
    }

    return state->buffer.ptr;
}

/// Parse an identifier from tokenizer, starting with character c.
static char* parse_identifier(struct tokenizer_state* state, int c, numeric_value_t* length) {
    struct location locationBackup;
    do {
        if (!STACK_PUSH(state->buffer, c))
            return NULL;
        locationBackup = state->location;
        c = tok_getc(state);
        if (c == TOKEN_ERROR)
            return NULL;
    } while (is_identifier_char(c));

    state->location = locationBackup;
    ungetc(c, state->fp);

    if (!STACK_PUSH(state->buffer, '\0'))
        return NULL;

    *length = state->buffer.used - 1;
    if (*length < 0 || (size_t)(*length) != state->buffer.used - 1) {
        localized_error(state->location, "Too long identifier");
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

    if (c == '0') {
        // next character determines the base, or zero
        struct location locationBackup = state->location;
        c = tok_getc(state);
        switch (c) {
        case TOKEN_ERROR:
            return -1;
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
                localized_error(locationBackup, "Leading zero in decimal integer literal");
                return -1;
            } else {
                ungetc(c, state->fp);
                state->location = locationBackup;
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
        struct location locationBackup = state->location;
        c = tok_getc(state);
        if (c == TOKEN_ERROR)
            return -1;
        else if (c == '_')
            continue;
        int d = parse_digit(c);
        if (d < 0 || d >= base) {
            if (haveDigits) {
                ungetc(c, state->fp);
                state->location = locationBackup;
                return ret;
            } else {
                localized_error(state->peekBuffer.location, "Base-%d numeric literal with no digits", base);
                return -1;
            }
        }

        bool overflow = __builtin_mul_overflow(ret, base, &ret);
        overflow = overflow || __builtin_add_overflow(ret, d, &ret);

        if (overflow) {
            localized_error(state->peekBuffer.location, "Numeric literal overflow");
            return -1;
        }
        haveDigits = true;
    }
}

/// Load a token into the peek buffer
static void load_next_token(struct tokenizer_state* state) {
    int c = tok_getc(state);

    while (c == '#' || is_skippable_whitespace(c)) {
        if (c == '#') { // Skip over comments and whitespace
            do {
                c = tok_getc(state);
            } while (c != TOKEN_EOF && c != TOKEN_ERROR && c != '\n');
            continue;
        }

        while (is_skippable_whitespace(c)) { // Skip over whitespace
            c = tok_getc(state);
        }
    }

    state->peekBuffer.location = state->location;
    state->peekBuffer.content = NULL;
    state->buffer.used = 0;

    if (c == TOKEN_ERROR)
        state->peekBuffer.type = TOKEN_ERROR;
    if (c == TOKEN_EOF)
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
        state->peekBuffer.type = TOKEN_STRING;
        state->peekBuffer.content = parse_string(state, &state->peekBuffer.contentNumeric);
        if (!state->peekBuffer.content)
            state->peekBuffer.type = TOKEN_ERROR;
    } else if (is_identifier_first_char(c)) {
        state->peekBuffer.type = TOKEN_IDENTIFIER;
        state->peekBuffer.content = parse_identifier(state, c, &state->peekBuffer.contentNumeric);
        if (!state->peekBuffer.content)
            state->peekBuffer.type = TOKEN_ERROR;
    }
    else if (isdigit(c)) {
        state->peekBuffer.type = TOKEN_NUMBER;
        state->peekBuffer.contentNumeric = parse_number(state, c);
        if (state->peekBuffer.contentNumeric < 0)
            state->peekBuffer.type = TOKEN_ERROR;
    }
    else {
        unexpected_character_error(state->location, c);
        state->peekBuffer.type = TOKEN_ERROR;
    }
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
    state->location.filename = filename;
    state->location.line = 1;
    state->location.column = 0;

    // Clear the state, so that a tokenizer that failed to open can still be safely passed
    // to close and it is a no-op.
    state->buffer.ptr = NULL;
    state->peekBuffer.type = TOKEN_ERROR;
    state->peekBuffer.content = NULL;

    state->fp = fopen(filename, "rb");
    if (!state->fp) {
        error("%s: Failed to open file", filename);
        return false;
    }

    if (!STACK_INIT(state->buffer, 32)) {
        fclose(state->fp);
        state->fp = NULL;
        return false;
    }

    load_next_token(state);

    return true;
}

void tokenizer_close(struct tokenizer_state* state) {
    if (state->fp)
        fclose(state->fp);
    state->fp = NULL;

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
