#pragma once

#include "tokenizer.h"

#include <stdint.h>
#include <stdbool.h>

struct section {
    char *name;
    uint16_t startAddress;
    uint16_t spc;
    uint16_t size;
    struct section* next;
};

struct symbol {
    char* name;
    uint16_t address;
    struct section* section; // Non-owning
    bool defined;
    struct symbol* next;
};

struct assembler_state {
    struct section* currentSection; // Non-owning
    struct symbol* symtable; // TODO: Use self-organizing list, just for fun (but measure before and after!)
    struct section* sectionTable; // Non-owning
    struct section* lastSection; // Needed for appending the sections, non-owning
    int pass;
};

bool assembler_state_init(struct assembler_state* state);
void assembler_state_deinit(struct assembler_state* state);
bool assembler_state_start_pass(int pass, struct assembler_state* state);
bool assemble(struct tokenizer_state* tokenizer, struct assembler_state* state);
bool assemble_multiple_files(int fileCount, char** filePaths, struct assembler_state* state);
bool assembler_output_word(uint16_t word, struct assembler_state* state);
bool get_symbol_value(struct token* idToken, struct assembler_state* state, uint16_t* ret);
bool assembler_state_enter_section(struct token* nameToken, struct assembler_state *state);
