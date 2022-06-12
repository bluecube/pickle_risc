#pragma once

struct token;
struct assembler_state;

bool process_pseudo_instruction(struct token* mnemonicToken, struct assembler_state* state, struct tokenizer_state* tokenizer);
