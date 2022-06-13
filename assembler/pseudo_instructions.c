#include "assembler.h"
#include "tokenizer.h"
#include "pseudo_instructions.h"
#include "printing.h"
#include "expressions.h"

#include <string.h>

#define SEP_ERROR 0
#define SEP_CONTINUE 1
#define SEP_FINISHED 2

static int parse_sep(struct tokenizer_state* tokenizer, bool canContinue) {
    struct token sep = get_token(tokenizer);

    switch (sep.type) {
    case TOKEN_ERROR:
        free_token(&sep);
        return SEP_ERROR;
    case TOKEN_EOL:
    case TOKEN_EOF:
        free_token(&sep);
        return SEP_FINISHED;
    default:
        if (sep.type == ',' && canContinue) {
            free_token(&sep);
            return SEP_CONTINUE;
        }
        localized_error(sep.location, "Unexpected input");
        free_token(&sep);
        return SEP_ERROR;
    }
}

static bool process_db(struct assembler_state *state, struct tokenizer_state *tokenizer) {
    struct token value = get_token(tokenizer);

    if (value.type == TOKEN_STRING) {
        for (numeric_value_t i = 1; i < value.contentNumeric; i += 2) {
            if (!assembler_output_word(
                value.content[i - 1] << 8 | value.content[i],
                state
            )) {
                free_token(&value);
                return false;
            }
        }
        if (value.contentNumeric & 1)
            if (!assembler_output_word(
                value.content[value.contentNumeric - 1] << 8,
                state
            )) {
                free_token(&value);
                return false;
            }
        free_token(&value);

        if (parse_sep(tokenizer, false) == SEP_FINISHED)
            return true;
        else
            return false;
    } else {
        unget_token(&value, tokenizer);

        uint16_t buffer = 0;
        bool bufferHasValue = false;

        while (true) {
            numeric_value_t v;
            if (!evaluate_expression(state, tokenizer, &v, NULL))
                return false;

            if (bufferHasValue) {
                buffer |= v & 0xff;
                if (!assembler_output_word(buffer, state))
                    return false;
                bufferHasValue = false;
            } else {
                buffer = (v & 0xff) << 8;
                bufferHasValue = true;
            }

            switch (parse_sep(tokenizer, true)) {
            case SEP_ERROR:
                return false;
            case SEP_FINISHED:
                if (bufferHasValue)
                    if (!assembler_output_word(buffer, state))
                        return false;
                return true;
            case SEP_CONTINUE:
                break;
            }
        }
    }
}

static bool process_dw(struct assembler_state *state, struct tokenizer_state *tokenizer) {
    while (true) {
        numeric_value_t v;
        if (!evaluate_expression(state, tokenizer, &v, NULL))
            return false;

        if (!assembler_output_word(v & 0xffff, state))
            return false;

        switch (parse_sep(tokenizer, true)) {
        case SEP_ERROR:
            return false;
        case SEP_FINISHED:
            return true;
        case SEP_CONTINUE:
            break;
        }
    }
}

static bool process_dd(struct assembler_state *state, struct tokenizer_state *tokenizer) {
    while (true) {
        numeric_value_t v;
        if (!evaluate_expression(state, tokenizer, &v, NULL))
            return false;

        unsigned_numeric_value_t vu = (unsigned_numeric_value_t)v;

        if (!assembler_output_word((vu >> 16) & 0xffff, state))
            return false;
        if (!assembler_output_word(vu & 0xffff, state))
            return false;

        switch (parse_sep(tokenizer, true)) {
        case SEP_ERROR:
            return false;
        case SEP_FINISHED:
            return true;
        case SEP_CONTINUE:
            break;
        }
    }
}

static bool process_include(struct assembler_state *state, struct tokenizer_state *tokenizer) {
    struct token pathToken = get_token(tokenizer);
    if (pathToken.type == TOKEN_ERROR) {
        free_token(&pathToken);
        return false;
    } else if (pathToken.type != TOKEN_STRING) {
        localized_error(pathToken.location, "Expected string literal");
        free_token(&pathToken);
        return false;
    }

    if (parse_sep(tokenizer, false) == SEP_ERROR) {
        free_token(&pathToken);
        return false;
    }

    struct tokenizer_state includedTokenizer;
    if (!tokenizer_open(pathToken.content, &includedTokenizer)) {
        free_token(&pathToken);
        return false;
    }

    struct section *sectionBackup = state->currentSection;
    bool result = assemble(&includedTokenizer, state);
    state->currentSection = sectionBackup;

    tokenizer_close(&includedTokenizer);
    free_token(&pathToken);
    return result;
}

static bool process_section(struct assembler_state *state, struct tokenizer_state *tokenizer) {
    struct token nameToken = get_token(tokenizer);
    if (nameToken.type == TOKEN_ERROR) {
        free_token(&nameToken);
        return false;
    } else if (nameToken.type != TOKEN_STRING) {
        localized_error(nameToken.location, "Expected string literal");
        free_token(&nameToken);
        return false;
    }

    if (parse_sep(tokenizer, false) == SEP_ERROR) {
        free_token(&nameToken);
        return false;
    }

    return assembler_state_enter_section(&nameToken, state);
}

bool process_pseudo_instruction(struct token* mnemonicToken, struct assembler_state* state, struct tokenizer_state* tokenizer) {
    bool ret;
    if (!strcmp(mnemonicToken->content, ".db"))
        ret = process_db(state, tokenizer);
    else if (!strcmp(mnemonicToken->content, ".dw"))
        ret = process_dw(state, tokenizer);
    else if (!strcmp(mnemonicToken->content, ".dd"))
        ret = process_dd(state, tokenizer);
    else if (!strcmp(mnemonicToken->content, ".include"))
        ret = process_include(state, tokenizer);
    else if (!strcmp(mnemonicToken->content, ".section"))
        ret = process_section(state, tokenizer);
    else {
        localized_error(mnemonicToken->location, "Invalid pseudo-instruction `%s`", mnemonicToken->content);
        ret = false;
    }

    free_token(mnemonicToken);
    return ret;
}
