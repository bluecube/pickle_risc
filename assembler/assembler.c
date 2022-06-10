#include "assembler.h"
#include "tokenizer.h"
#include "util.h"
#include "instructions.h"
#include "printing.h"
#include "expressions.h"

#include <stdlib.h>
#include <string.h>
#include <assert.h>

void assembler_state_init(struct assembler_state* state) {
    state->symtable = NULL;
}

void assembler_state_before_pass(int pass, struct assembler_state* state) {
    (void)pass;
    state->pc = 0;
}

void assembler_state_deinit(struct assembler_state* state) {
    while (state->symtable) {
        struct symbol *tmp = state->symtable->next;
        free(state->symtable->name);
        free(state->symtable);
        state->symtable = tmp;
    }
}

static struct symbol* lookup_symbol(const char* name, struct assembler_state* state) {
    struct symbol *p = state->symtable;
    while (p) {
        if (!strcmp(name, p->name))
            return p;
        p = p->next;
    }
    return NULL;
}

/// Lookup a symbol in assembler's symbol table and return it, either pre-existing one
/// or a new undefined symbol. Takes ownership of name.
/// Returns NULL on error.
struct symbol* lookup_or_create_symbol(char* name, struct assembler_state* state) {
    struct symbol *sym = lookup_symbol(name, state);
    if (sym) {
        free(name);
        return sym;
    }

    sym = malloc_with_msg(sizeof(struct symbol), "symbol table entry");
    if (!sym) {
        free(name);
        return NULL;
    }
    sym->name = name;
    sym->address = 0;
    sym->defined = false;
    sym->next = state->symtable;
    state->symtable = sym;

    return sym;
}

/// Process a label definition, takes ownership of name
static bool define_symbol(struct token* nameToken, struct assembler_state* state) {
    struct location loc = nameToken->location;
    char* name = free_token_move_content(nameToken);
    struct symbol* sym = lookup_or_create_symbol(name, state);

    if (!sym)
        return false;

    if (sym->defined) {
        localized_error(loc, "Symbol `%s` is already defined", sym->name);
        return false;
    }

    sym->address = state->pc;
    sym->defined = true;

    return true;
}

/// Parse general purpose register name from the next token into register number
/// or return negative value and print error
static int16_t parse_gpr(struct tokenizer_state* tokenizer) {
    struct token tok = get_token(tokenizer);
    int16_t ret = -1;

    if (
        tok.type == TOKEN_IDENTIFIER &&
        tok.contentNumeric == 2 &&
        tok.content[0] == 'r'
    ) {
        int digit = parse_digit(tok.content[1]);
        if (digit >= 0 && digit < 8)
            ret = digit;
    }

    if (ret < 0)
        localized_error(tok.location, "Expected register name (r0-r7)");

    free_token(&tok);

    return ret;
}

/// Parse control register name from the next token into register number
/// or return negative value and print error
static int16_t parse_cr(struct tokenizer_state* tokenizer) {
    static const char* cr_names[] = {
        "Status", "Tmp1", "Tmp2", "ContextId",
        "IntCause", "IntPc", "MMUAddr", "MMUData"
    };

    struct token tok = get_token(tokenizer);
    int16_t ret = -1;

    if (tok.type == TOKEN_IDENTIFIER) {
        const int nameCount = sizeof(cr_names) / sizeof(cr_names[0]);
        for (int i = 0; i < nameCount; ++i) {
            if (!strcmp(tok.content, cr_names[i])) {
                ret = i;
                break;
            }
        }
    }

    if (ret < 0)
        localized_error(tok.location, "Expected control register name");

    free_token(&tok);

    return ret;
}

/// Parse a number from the input and return it as an unsigned number suitable
/// for output into an instruction code
/// @return -1 on error
static int16_t parse_number_for_instruction(bool inputSigned, unsigned size, struct assembler_state* state, struct tokenizer_state* tokenizer) {
    numeric_value_t number;
    struct location location;

    if (!evaluate_expression(state, tokenizer, &number, &location))
        return -1;

    assert(size < 15); // We must fit into the output 16 bits, with negative value reserved

    numeric_value_t min = 0;
    numeric_value_t max = 1 << size;

    if (inputSigned) {
        min = -(max / 2);
        max = (max / 2) - 1;
    }

    if (number < min || number > max) {
        localized_error(location, "Value %" NUMERIC_VALUE_FORMAT " out of range (%" NUMERIC_VALUE_FORMAT " .. %" NUMERIC_VALUE_FORMAT ")", number, min, max);
        return -1;
    }

    if (number >= 0)
        return number;
    else
        return (1 << size) + number;
}

static bool process_instruction(struct token *mnemonicToken, struct assembler_state* state, struct tokenizer_state* tokenizer) {
    struct instruction* instruction = instructions;
    while (instruction->mnemonic) {
        if (!strcmp(mnemonicToken->content, instruction->mnemonic))
            break;
        ++instruction;
    }
    if (!instruction->mnemonic) {
        localized_error(mnemonicToken->location, "Invalid instruction %s", mnemonicToken->content);
        free_token(mnemonicToken);
        return false;
    }

    free_token(mnemonicToken);

    struct instruction_argument* arg = instruction->args;

    uint16_t encoding = instruction->encoding;

    while (arg->type != INSTRUCTION_ARG_NONE) {
        int16_t argValue;

        switch (arg->type) {
        case INSTRUCTION_ARG_GPR:
            argValue = parse_gpr(tokenizer);
            break;
        case INSTRUCTION_ARG_CR:
            argValue = parse_cr(tokenizer);
            break;
        case INSTRUCTION_ARG_SIGNED:
        case INSTRUCTION_ARG_UNSIGNED:
            argValue = parse_number_for_instruction(
                arg->type == INSTRUCTION_ARG_SIGNED,
                arg->size,
                state,
                tokenizer
            );
            break;
        default:
            assert(false);
        }

        if (argValue < 0) {
            // Parsing the argument failed
            return false;
        }

        assert(argValue >> arg->size == 0);
        assert(arg->shift + arg->size <= 16);
        encoding |= argValue << arg->shift;

        ++arg;

        struct token separator = get_token(tokenizer);
        bool lastArg = (arg->type == INSTRUCTION_ARG_NONE);
        bool error = (separator.type == TOKEN_ERROR);
        bool lineEnd = (separator.type == TOKEN_EOF || separator.type == TOKEN_EOL);
        bool comma = (separator.type == ',');
        struct location loc = separator.location;
        free_token(&separator);

        if (error)
            return false;
        else if (comma && lastArg) {
            localized_error(loc, "Extra instruction parameter");
            return false;
        }
        else if (lineEnd && !lastArg) {
            localized_error(loc, "Missing instruction parameters");
            return false;
        }
        else if (!comma && !lineEnd) {
            localized_error(loc, "Unexpected input");
            return false;
        }
    }

    printf("0x%04x: 0x%04x\n", state->pc, encoding);
    state->pc += 1;

    return true;
}

static bool assemble(int pass, struct tokenizer_state* tokenizer, struct assembler_state* state) {
    (void)pass;
    while (true) {
        struct token token1 = get_token(tokenizer);
        if (token1.type == TOKEN_ERROR)
            return false;
        else if (token1.type == TOKEN_EOF)
            return true;
        else if (token1.type == TOKEN_EOL)
            continue; // Empty line
        else if (token1.type != TOKEN_IDENTIFIER) {
            localized_error(token1.location, "Expected identifier");
            return false;
        }

        struct token token2 = get_token(tokenizer);
        if (token2.type == ':') {
            free_token(&token2);
            if (!define_symbol(&token1, state))
                return false;
        } else {
            unget_token(&token2, tokenizer);
            if (!process_instruction(&token1, state, tokenizer))
                return false;
        }
    }
}

bool assemble_multiple_files(int pass, int fileCount, char** filePaths, struct assembler_state* state) {
    struct tokenizer_state tokenizer;
    for (int i = 0; i < fileCount; ++i) {
        if (!tokenizer_open(filePaths[i], &tokenizer))
            return false;
        bool result = assemble(pass, &tokenizer, state);
        tokenizer_close(&tokenizer);

        if (!result)
            return false;
    }

    return true;
}
