//! This is a simple precedence climbing expression pareser/evaluator.
//! To make it easier to test, it plugs into the assembler state only through one function
//! `get_symbol`, that returns symbol value given its name, or None if the symbol is not defined.

use std::borrow::Cow;

use crate::assembler::{
    lexer::Token,
    parser::{one_token, ParseResult},
    files::{FileTokens, Location},
    Value, AsmError,
};

pub fn expression<'a>(
    tokens: FileTokens<'a>,
    get_symbol: &impl Fn(&str) -> Option<Value>,
) -> ParseResult<'a, Value> {
    let (first_value, first_location, tokens) = value(tokens, get_symbol)?;
    main(first_value, first_location, 0, tokens, get_symbol)
}

/// Parse and evaluate a single value that has lower precedence than all binary operators.
/// Handles atoms (number / identifier), parenthesised expressions and unary operators.
fn value<'a>(
    tokens: FileTokens<'a>,
    get_symbol: &impl Fn(&str) -> Option<Value>,
) -> ParseResult<'a, Value> {
    static EXPECTED: &str = "`(`, `+`, `-`, `!`, `~`, identifier or number";
    match tokens.first() {
        Some((Token::Number(n), location)) => Ok((*n, location, tokens.rest())),
        Some((Token::Identifier(ident), location)) => {
            let v = get_symbol(&ident)
                .ok_or_else(|| AsmError::UndefinedSymbol { location })?;
            Ok((v, location, tokens.rest()))
        }
        Some((Token::LParen, location1)) => {
            let (v, _, tokens) = expression(tokens.rest(), get_symbol)?;
            let (_, location2, tokens) = one_token(tokens, &Token::RParen)?;
            Ok((v, location1.extend_to(&location2), tokens))
        }
        // Unary operators:
        Some((Token::Plus, location1)) => {
            let (v, location2, tokens) = value(tokens.rest(), get_symbol)?;
            Ok((v, location1.extend_to(&location2), tokens))
        }
        Some((Token::Minus, location1)) => {
            let (v, location2, tokens) = value(tokens.rest(), get_symbol)?;
            let result_location = location1.extend_to(&location2);
            let result = v.checked_neg().ok_or_else(|| AsmError::ValueOutOfRange {
                location: result_location,
            })?;
            Ok((result, result_location, tokens))
        }
        Some((Token::Not, location1)) => {
            let (v, location2, tokens) = value(tokens.rest(), get_symbol)?;
            Ok(((v == 0).into(), location1.extend_to(&location2), tokens))
        }
        Some((Token::BitNot, location1)) => {
            let (v, location2, tokens) = value(tokens.rest(), get_symbol)?;
            Ok((!v, location1.extend_to(&location2), tokens))
        }
        Some((_, location)) => Err(AsmError::UnexpectedToken {
            expected: Cow::Borrowed(EXPECTED),
            location,
        }),
        None => Err(AsmError::UnexpectedEof {
            expected: Cow::Borrowed(EXPECTED),
        }),
    }
}

fn main<'a>(
    lhs: Value,
    lhs_location: Location,
    min_precedence: u32,
    tokens: FileTokens<'a>,
    get_symbol: &impl Fn(&str) -> Option<Value>,
) -> ParseResult<'a, Value> {
    let mut tokens = tokens;
    let mut lhs = lhs;
    let mut lhs_location = lhs_location;

    loop {
        let Some((op, _)) = tokens.first() else {
            break;
        };
        let Some(op_precedence) = binary_operator_precedence(op).filter(|precedence| precedence >= &min_precedence) else {
            break;
        };
        tokens = tokens.rest();

        let (mut rhs, mut rhs_location, t) = value(tokens, get_symbol)?;
        tokens = t;
        loop {
            if tokens.first().and_then(|(t, _)| binary_operator_precedence(t)).filter(|precedence| precedence > &op_precedence).is_none() {
                break;
            }
            (rhs, rhs_location, tokens) = main(rhs, rhs_location, op_precedence + 1, tokens, get_symbol)?;
        }

        (lhs, lhs_location) = eval_binary_operator(lhs, lhs_location, rhs, rhs_location, &op)?;
    }

    Ok((lhs, lhs_location, tokens))
}

/// Return binary operator precedence for a token, or None if the token doesn't
/// correspond to a binary operator.
/// Loosely based on C operator precedence table.
fn binary_operator_precedence(token: &Token) -> Option<u32> {
    use Token::*;
    match token {
        BitOr => Some(0),
        BitXor => Some(1),
        BitAnd => Some(2),
        Eq | Neq | Lt | Gt | Le | Ge => Some(3),
        Shl | Shr => Some(4),
        Plus | Minus => Some(5),
        Asterisk | Slash | Percent => Some(6),
        _ => None,
    }
}

fn eval_binary_operator(
    lhs: Value,
    lhs_location: Location,
    rhs: Value,
    rhs_location: Location,
    operator: &Token,
) -> Result<(Value, Location), AsmError> {
    fn from_bool(b: bool) -> Option<Value> {
        Some(b.into())
    }

    let v = match operator {
        Token::Asterisk => lhs.checked_mul(rhs),
        Token::Slash => lhs.checked_div(rhs),
        Token::Percent => lhs.checked_rem(rhs),
        Token::Plus => lhs.checked_add(rhs),
        Token::Minus => lhs.checked_sub(rhs),
        Token::Shl => {
            lhs.checked_shl(
                rhs.try_into()
                    .map_err(|_| AsmError::NegativeShiftAmmount {
                        location: rhs_location.clone(),
                    })?,
            )
        }
        Token::Shr => {
            lhs.checked_shr(
                rhs.try_into()
                    .map_err(|_| AsmError::NegativeShiftAmmount {
                        location: rhs_location.clone(),
                    })?,
            )
        }
        Token::Eq => from_bool(lhs == rhs),
        Token::Neq => from_bool(lhs != rhs),
        Token::Lt => from_bool(lhs < rhs),
        Token::Gt => from_bool(lhs > rhs),
        Token::Le => from_bool(lhs <= rhs),
        Token::Ge => from_bool(lhs >= rhs),
        Token::LogicalAnd => from_bool((lhs != 0) & (rhs != 0)),
        Token::LogicalOr => from_bool((lhs != 0) | (rhs != 0)),
        Token::BitAnd => Some(lhs & rhs),
        Token::BitXor => Some(lhs ^ rhs),
        Token::BitOr => Some(lhs | rhs),
        _ => unreachable!(),
    };

    let location = lhs_location.extend_to(&rhs_location);

    match v {
        Some(v) => Ok((v, location)),
        None => Err(AsmError::ValueOutOfRange { location }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembler::files::SnippetTokenizer;
    use assert_matches::assert_matches;
    use test_case::test_case;

    fn no_symbols(_: &str) -> Option<Value> {
        None
    }

    /// Tests that parsing the expected input as a value does not fail and
    /// results in expected output.
    /// The input is first tokenized, with '+' appended to verify that
    /// the value doesnt consume more than it should
    #[test_case("42", 42 ; "number")]
    #[test_case("2_147_483_647", 2_147_483_647 ; "max_number")]
    #[test_case("foo", 13 ; "identifier")]
    #[test_case("(3)", 3 ; "parenthesis")]
    #[test_case("((4))", 4 ; "nested_parenthesis")]
    #[test_case("(3 + 2)", 5 ; "parenthesis_expression")]
    #[test_case("-43", -43 ; "unary_minus")]
    #[test_case("-2_147_483_647", -2_147_483_647 ; "minus_max_number")]
    #[test_case("+44", 44 ; "unary_plus")]
    #[test_case("!42", 0 ; "not_true")]
    #[test_case("!0", 1 ; "not_false")]
    #[test_case("!foo", 0 ; "not_foo")]
    #[test_case("~1", -2 ; "bit_not")]
    #[test_case("-(18)", -18 ; "minus_parenthesis")]
    #[test_case("--19", 19 ; "double_unary_minus")]
    #[test_case("+-~!0", 2 ; "all_unary")]
    fn value_happy_path(input: &str, expected: Value) {
        fn get_symbol(s: &str) -> Option<i32> {
            if s == "foo" {
                Some(13)
            } else {
                None
            }
        }
        let tokenizer = SnippetTokenizer::new(format!("{input}+"));
        let expected_location = tokenizer.location(0, input.len());
        let (result, location, rest) = value(tokenizer.tokens(), &get_symbol).unwrap();
        assert_eq!(result, expected);
        assert_eq!(location, expected_location);
        assert_matches!(rest.first(), Some((Token::Plus, _)));
    }

    #[test_case("" ; "eof")]
    #[test_case("+" ; "unexpected_token_1")]
    #[test_case(";" ; "unexpected_token_2")]
    #[test_case("9999999999999999999999" ; "error_token")]
    #[test_case("bar" ; "undefined_symbol")]
    #[test_case("(1" ; "missing_rparen")]
    #[test_case("()" ; "empty_parentheses")]
    #[test_case(":" ; "unexpected_token")]
    fn value_error(input: &str) {
        let tokenizer = SnippetTokenizer::new(input.to_owned());
        let _err = value(tokenizer.tokens(), &no_symbols).unwrap_err();
        // TODO: Check error content
    }

    #[test_case("1234", 1234; "trivial")]
    #[test_case("1+1", 2; "simple")]
    #[test_case("1+2*3", 7; "simple_prio")]
    #[test_case("3*2+1", 7; "simple_prio2")]
    #[test_case("1+1 2+2", 2; "junk_after1")]
    #[test_case("1+1:", 2; "junk_after2")]
    #[test_case("5*2/3", 3; "left_associativity")]
    #[test_case("5*2+3&0xfe", 12; "increasing_precedence")]
    #[test_case("1|2+3*4", 15; "decreasing_precedence")]
    #[test_case("(1 << 8) - 1", 255; "bitmask1")]
    #[test_case("0xabcd & ~((1 << 8) - 1)", 0xab00; "bitmask2")]
    #[test_case("2*3 - 4*5 + 6/3", -12; "mul_div_add_sub")]
    #[test_case("1 + 1 == 3 - 1", 1; "equals")]
    #[test_case("-2_147_483_647 - 1", -2_147_483_648; "minimum_value")]
    #[test_case("0b0100 | 0b1001 ^ 0b1100 & 0b1010", 0b0101; "bitwise_operations")]
    fn expression_happy_path(input: &str, expected: Value) {
        let tokenizer = SnippetTokenizer::new(input.to_owned());
        let (result, _location, rest) = expression(tokenizer.tokens(), &no_symbols).unwrap();
        assert_eq!(result, expected);
        //assert_eq!(rest.first(), None);
    }

    #[test_case("1+" ; "missing_rhs")]
    #[test_case("/1" ; "missing_lhs")]
    #[test_case("1/0" ; "divide_by_zero")]
    #[test_case("-(-2_147_483_647 - 1)" ; "neg_overflow")]
    #[test_case("1 << -2"; "shl_neg")]
    #[test_case("1 >> -2"; "shr_neg")]
    #[test_case("0xffffff * 0xffffff"; "mul_overflow")]
    fn expression_error(input: &str) {
        let tokenizer = SnippetTokenizer::new(input.to_owned());
        let _err = expression(tokenizer.tokens(), &no_symbols).unwrap_err();
        // TODO: Check error content
    }

    #[test_case("2 * 4", 8; "mul")]
    #[test_case("4 / 2", 2; "div")]
    #[test_case("11 % 4", 3; "modulus")]
    #[test_case("1 + 1", 2; "add")]
    #[test_case("1 - 9", -8; "sub")]
    #[test_case("0b110010 << 4", 0b1100100000; "shl")]
    #[test_case("0b110010 >> 3", 0b110; "shr")]
    #[test_case("1 == 2", 0; "equals")]
    #[test_case("1 != 2", 1; "not_equals")]
    #[test_case("1 < 2", 1; "lt_1")]
    #[test_case("2 < 2", 0; "lt_2")]
    #[test_case("1 <= 2", 1; "le_1")]
    #[test_case("2 <= 2", 1; "le_2")]
    #[test_case("2 > 1", 1; "gt_1")]
    #[test_case("2 > 2", 0; "gt_2")]
    #[test_case("2 >= 1", 1; "ge_1")]
    #[test_case("2 >= 2", 1; "ge_2")]
    #[test_case("0b110010 & 0b101010", 0b100010; "bit_and")]
    #[test_case("0b110010 | 0b101010", 0b111010; "bit_or")]
    #[test_case("0b110010 ^ 0b101010", 0b011000; "bit_xor")]
    #[test_case("0 && 0", 0; "and_1")]
    #[test_case("0 && 100", 0; "and_2")]
    #[test_case("100 && 0", 0; "and_3")]
    #[test_case("100 && 100", 1; "and_4")]
    #[test_case("0 || 0", 0; "or_1")]
    #[test_case("0 || 100", 1; "or_2")]
    #[test_case("100 || 0", 1; "or_3")]
    #[test_case("100 || 100", 1; "or_4")]
    fn evaluate_binary_operator(input: &str, expected: Value) {
        let tokenizer = SnippetTokenizer::new(input.to_owned());
        let tokens = tokenizer.tokens();
        let (lhs, lhs_location) =
            assert_matches!(tokens.first(), Some((Token::Number(n), location)) => (n, location));
        let tokens = tokens.rest();
        let op = tokens.first().unwrap().0;
        let tokens = tokens.rest();
        let (rhs, rhs_location) =
            assert_matches!(tokens.first(), Some((Token::Number(n), location)) => (n, location));

        let (result, location) = eval_binary_operator(*lhs, lhs_location, *rhs, rhs_location, &op).unwrap();

        assert_eq!(result, expected);
        assert_eq!(location, tokenizer.location(0, input.len()));
    }
}
