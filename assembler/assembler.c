#include "assembler.h"
#include "tokenizer.h"
#include "instructions.h"
#include "expressions.h"
#include "pseudo_instructions.h"

#include "../common/util.h"
#include "../common/printing.h"

#include <stdlib.h>
#include <string.h>
#include <assert.h>

#define DEFAULT_SECTION_NAME ".text"

static bool push_identifier_to_buffer(struct token *identifier, print_buffer_t *buffer) {
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


/// Create a symbol in the symbol table of assembler state.
/// Takes ownership of name.
/// Doesn't check that the symbol doesn't alreday exist.
/// @return new symbol or NULL on error.
static struct symbol* create_symbol(char* name, struct assembler_state* state) {
    struct symbol* sym = malloc_with_msg(sizeof(struct symbol), "symbol table entry");
    if (!sym) {
        free(name);
        return NULL;
    }
    sym->name = name;
    sym->address = 0;
    sym->section = NULL;
    sym->defined = false;
    sym->next = state->symtable;
    state->symtable = sym;

    return sym;
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

/// Create a section in the section table of assembler state.
/// Takes ownership of name.
/// Doesn't check that the section doesn't alreday exist.
/// @return new symbol or NULL on error.
struct section *create_section(char *name, struct assembler_state *state) {
    struct section *section = malloc_with_msg(sizeof(struct section), "section table entry");
    if (!section) {
        free(name);
        return NULL;
    }

    section->name = name;
    section->spc = 0;
    section->startAddress = 0;
    section->size = 0;
    section->next = NULL;
    if (!state->sectionTable) {
        assert(!state->lastSection);
        state->sectionTable = section;
    } else {
        assert(state->lastSection);
        state->lastSection->next = section;
    }
    state->lastSection = section;

    return section;
}

static struct section* lookup_section(const char* name, struct assembler_state* state) {
    struct section *p = state->sectionTable;
    while (p) {
        if (!strcmp(name, p->name)) {
            return p;
        }
        p = p->next;
    }
    return NULL;
}

bool assembler_state_enter_section(struct token* nameToken, struct assembler_state *state) {
    struct location loc = nameToken->location;
    char* name = free_token_move_content(nameToken);

    struct section *section = lookup_section(name, state);

    if (!section) {
        if (state->pass == 1) {
            section = create_section(name, state);
            if (!section)
                return false;
        } else {
            localized_error(loc, "Section `%s` was not defined in first pass", name);
            free(name);
            return false;
        }
    } else
        free(name);

    state->currentSection = section;
    return true;
}

bool assembler_state_init(const char *outputFile, bool verbose, struct assembler_state* state) {
    if (verbose) {
        if (!STACK_INIT(state->verbosePrintBuffer, 16))
            return false;
    } else
        state->verbosePrintBuffer.ptr = NULL; // Doubles as a verbosity flag

    state->symtable = NULL;
    state->sectionTable = NULL;

    char* ownedSectionName = strdup_with_msg(DEFAULT_SECTION_NAME, "default section name");
    if (!ownedSectionName) {
        STACK_DEINIT(state->verbosePrintBuffer);
        return false;
    }
    state->lastSection = NULL;
    state->currentSection = create_section(ownedSectionName, state);
    if (!state->currentSection) {
        STACK_DEINIT(state->verbosePrintBuffer);
        return false;
    }

    if (!ihex_output_open(&state->output, outputFile)) {
        assembler_state_deinit(state);
        return false;
    }

    return true;
}

bool assembler_state_start_pass(int pass, struct assembler_state* state) {
    state->pass = pass;

    uint16_t sectionStart = 0;
    struct section *section = state->sectionTable;
    while (section) {
        section->size = section->spc;
        section->startAddress = sectionStart;
        sectionStart += section->size;

        if (pass == 2 && state->verbosePrintBuffer.ptr)
            printf(
                "Section `%s`: 0x%04x - 0x%04x\n",
                section->name,
                section->startAddress,
                section->startAddress + section->size
            );

        section->spc = 0;
        section = section->next;
    }

    return true;
}

bool assembler_state_deinit(struct assembler_state* state) {
    while (state->symtable) {
        struct symbol *tmp = state->symtable->next;
        free(state->symtable->name);
        free(state->symtable);
        state->symtable = tmp;
    }
    while (state->sectionTable) {
        struct section *tmp = state->sectionTable->next;
        free(state->sectionTable->name);
        free(state->sectionTable);
        state->sectionTable = tmp;
    }
    state->lastSection = NULL;
    state->currentSection = NULL;

    STACK_DEINIT(state->verbosePrintBuffer);

    return ihex_output_close(&state->output);
}

/// Process a label definition, takes ownership of nameToken
static bool define_symbol(struct token* nameToken, struct assembler_state* state) {
    struct location loc = nameToken->location;
    char* name = free_token_move_content(nameToken);

    struct symbol* sym = lookup_symbol(name, state);

    uint16_t address = state->currentSection->spc;
    struct section *section = state->currentSection;

    if (state->pass == 1) {
        if (!sym) {
            sym = create_symbol(name, state);
            if (!sym)
                return false;
            sym->defined = true;
            sym->address = address;
            sym->section = section;
        } else  if (!sym->defined) {
            free(name);
            sym->defined = true;
            sym->address = address;
            sym->section = section;
        } else if (sym->defined) {
            free(name);
            localized_error(loc, "Redefinition of symbol `%s`", sym->name);
            return false;
        }
    } else if (state->pass == 2) {
        free(name);
        if (!sym || !sym->defined) {
            localized_error(loc, "Symbol `%s` was not defined in first pass", sym->name);
            return false;
        } else if (sym->address != address || sym->section != section) {
            localized_error(
                loc,
                "Symbol `%s` changed address (pass 1: 0x%" PRIx16 " in section `%s`, pass2: 0x%" PRIx16 " in section `%s`)",
                sym->name,
                sym->address, sym->section->name,
                address, section->name
            );
            return false;
        }
    } else
        assert(false);

    return true;
}

/// Load value of a symbol defined by identifier to ret.
/// Takes ownership of token.
/// @return True iff successful.
bool get_symbol_value(struct token* nameToken, struct assembler_state* state, uint16_t* ret) {
    struct location location = nameToken->location;
    char* name = free_token_move_content(nameToken);

    struct symbol* sym = lookup_symbol(name, state);

    if (state->pass == 1) {
        if (sym) {
            free(name);
        } else {
            sym = create_symbol(name, state);
            if (!sym)
                return false;
        }
    } else if (state->pass == 2) {
        free(name);
        if (!sym || !sym->defined) {
            localized_error(location, "Symbol `%s` was not defined", sym->name);
            return false;
        }
    } else
        assert(false);

    uint16_t sectionAddress = 0;
    if (sym->section)
        sectionAddress = sym->section->startAddress;

    *ret = sectionAddress + sym->address;
    return true;
}

/// Parse general purpose register name from the next token into register number
/// or return negative value and print error
static int16_t parse_gpr(struct tokenizer_state *tokenizer, struct assembler_state *state) {
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

    if (!push_identifier_to_buffer(&tok, &(state->verbosePrintBuffer))) {
        free_token(&tok);
        return false;
    }

    free_token(&tok);

    return ret;
}

/// Parse control register name from the next token into register number
/// or return negative value and print error
static int16_t parse_cr(struct tokenizer_state *tokenizer, struct assembler_state *state) {
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

    if (!push_identifier_to_buffer(&tok, &(state->verbosePrintBuffer))) {
        free_token(&tok);
        return false;
    }

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

    if (!printf_to_buffer(&(state->verbosePrintBuffer), "%" NUMERIC_VALUE_FORMAT, number))
        return -1;

    if (number >= 0)
        return number;
    else
        return (1 << size) + number;
}

static bool process_instruction(struct token *mnemonicToken, struct assembler_state* state, struct tokenizer_state* tokenizer) {
    if (mnemonicToken->content[0] == '.')
        return process_pseudo_instruction(mnemonicToken, state, tokenizer);

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

    state->verbosePrintBuffer.used = 0; // Clean the buffer for this instruction
    push_identifier_to_buffer(mnemonicToken, &(state->verbosePrintBuffer));

    free_token(mnemonicToken);

    struct instruction_argument* arg = instruction->args;

    uint16_t encoding = instruction->encoding;

    while (arg->type != INSTRUCTION_ARG_NONE) {
        int16_t argValue;

        if (state->verbosePrintBuffer.ptr)
            STACK_PUSH(state->verbosePrintBuffer, ' ');

        switch (arg->type) {
        case INSTRUCTION_ARG_GPR:
            argValue = parse_gpr(tokenizer, state);
            break;
        case INSTRUCTION_ARG_CR:
            argValue = parse_cr(tokenizer, state);
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

    int32_t outputAddress = assembler_output_word(encoding, state);
    if (outputAddress < 0)
        return false;

    if (state->verbosePrintBuffer.ptr && state->pass == 2) {
        STACK_PUSH(state->verbosePrintBuffer, '\0');
        printf("%04" PRIx32 ": %s\n", outputAddress, state->verbosePrintBuffer.ptr);
    }

    return true;
}

bool assemble(struct tokenizer_state* tokenizer, struct assembler_state* state) {
    state->currentSection = lookup_section(DEFAULT_SECTION_NAME, state);
    assert(state->currentSection);
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
            free_token(&token1);
            return false;
        }

        if (peek_token(tokenizer)->type == ':') {
            skip_token(tokenizer);
            if (!define_symbol(&token1, state))
                return false;
        } else
            if (!process_instruction(&token1, state, tokenizer))
                return false;
    }
}

bool assemble_multiple_files(int fileCount, char** filePaths, struct assembler_state* state) {
    struct tokenizer_state tokenizer;
    for (int i = 0; i < fileCount; ++i) {
        if (!tokenizer_open(filePaths[i], &tokenizer))
            return false;
        bool result = assemble(&tokenizer, state);
        tokenizer_close(&tokenizer);

        if (!result)
            return false;
    }

    return true;
}

int32_t assembler_output_word(uint16_t word, struct assembler_state* state) {
    uint16_t wordAddress = state->currentSection->startAddress + state->currentSection->spc;
    if (state->pass == 2) {
        uint16_t address = wordAddress << 1;

        if (!ihex_output_byte(&state->output, address, (word >> 8) & 0xff))
            return -1;
        if (!ihex_output_byte(&state->output, address + 1, word & 0xff))
            return -1;
    }
    state->currentSection->spc += 1;
    return wordAddress;
}
