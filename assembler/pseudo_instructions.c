#include "assembler.h"
#include "tokenizer.h"
#include "pseudo_instructions.h"
#include "printing.h"
#include "expressions.h"

#include <string.h>

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

        struct token sep = get_token(tokenizer);
        switch (sep.type) {
        case TOKEN_ERROR:
            free_token(&sep);
            return false;
        case TOKEN_EOL:
        case TOKEN_EOF:
            return true;
        default:
            localized_error(sep.location, "Unexpected input");
            free_token(&sep);
            return false;
        }
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

            struct token sep = get_token(tokenizer);

            switch (sep.type) {
            case TOKEN_ERROR:
                free_token(&sep);
                return false;
            case TOKEN_EOL:
            case TOKEN_EOF:
                free_token(&sep);
                if (bufferHasValue)
                    if (!assembler_output_word(buffer, state))
                        return false;
                return true;
            case ',':
                free_token(&sep);
                break;
            default:
                localized_error(sep.location, "Unexpected input");
                free_token(&sep);
                return false;
            }
        }
    }
}

bool process_pseudo_instruction(struct token* mnemonicToken, struct assembler_state* state, struct tokenizer_state* tokenizer) {
    bool ret;
    if (!strcmp(mnemonicToken->content, ".db"))
        ret = process_db(state, tokenizer);
    /*else if (!strcmp(mnemonicToken->content, ".dw"))
        ret = process_dw(state, tokenizer);*/
    else {
        localized_error(mnemonicToken->location, "Invalid pseudo-instruction `%s`", mnemonicToken->content);
        ret = false;
    }

    free_token(mnemonicToken);
    return ret;
}
