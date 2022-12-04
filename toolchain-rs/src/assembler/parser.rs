use std::borrow::Cow;

use thiserror::Error;

use crate::assembler::lexer::{Span, Token, TokensIter};
use crate::assembler::state::{ParseState, ScopeId, Symbol, Value};
use crate::instruction::{ControlRegister, Gpr, Instruction};

pub type ParserResult<V> = Result<V, ParseError>;

pub fn top<'a>(state: &mut ParseState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    scope_content(state, tokens);

    match tokens.next() {
        Some((_, span)) => Err(ParseError::UnexpectedToken {
            expected: Cow::Borrowed("`{`, `;`, `\\n`, identifier or end of file"),
            span,
        }),
        None => Ok(()),
    }
}

/// Parses a label or a labeled scope
fn label<'a>(state: &mut ParseState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    let (id, span) = identifier(tokens)?;
    one_token(tokens, Token::Colon)?;

    match tokens.peek() {
        Some((Token::LBrace, _)) => {
            let scope_id = scope_start(state, tokens)?;
            state.define_symbol(id, state.current_pc_symbol(Some(scope_id), span))?;
            scope_content(state, tokens)?;
            scope_end(state, tokens)?;
        }
        _ => {
            state.define_symbol(id, state.current_pc_symbol(None, span))?;
        }
    }

    Ok(())
}

fn assignment<'a>(state: &mut ParseState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    let (id, _) = identifier(tokens)?;
    one_token(tokens, Token::Assign)?;

    let (value, span) =
        expr::expression(tokens, &|symbol_name| state.get_symbol_value(symbol_name))?;
    state.define_symbol(
        id,
        Symbol::Free {
            value,
            defined_at: span,
        },
    )?;

    Ok(())
}

fn anonymous_scope<'a>(state: &mut ParseState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    scope_start(state, tokens)?;
    scope_content(state, tokens)?;
    scope_end(state, tokens)?;
    Ok(())
}

fn scope_start<'a>(state: &mut ParseState, tokens: &mut TokensIter<'a>) -> ParserResult<ScopeId> {
    let _span = one_token(tokens, Token::LBrace)?;
    Ok(state.push_scope())
}

fn scope_content<'a>(state: &mut ParseState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    loop {
        tokens.reset_peek();
        match tokens.peek() {
            Some(&(Token::Identifier(ident), _)) => match tokens.peek() {
                // Peek second token!
                Some((Token::Colon, _)) => label(state, tokens)?,
                Some((Token::Assign, _)) => assignment(state, tokens)?,
                _ if ident.starts_with('.') => pseudo_instruction(state, tokens)?,
                _ => instruction(state, tokens)?,
            },
            Some((Token::LBrace, _)) => anonymous_scope(state, tokens)?,
            Some((Token::Eol, _)) | Some((Token::Semicolon, _)) => continue, // Skip empty lines
            _ => return Ok(()),
        }
    }
}

fn scope_end<'a>(state: &mut ParseState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    state.pop_scope();
    one_token(tokens, Token::RBrace)?;
    Ok(())
}

fn instruction<'a>(state: &mut ParseState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    use ux::*;
    let (mnemonic, span) = identifier(tokens)?;
    let instruction = include!(concat!(env!("OUT_DIR"), "/parse_asm_match.rs"))
        .ok_or_else(|| ParseError::UnexpectedInstructionMnemonic { span })?;
    todo!();
    Ok(())
}

fn pseudo_instruction<'a>(state: &mut ParseState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    let (mnemonic, span) = identifier(tokens)?;
    match mnemonic {
        ".db" => todo!(),
        ".dw" => todo!(),
        ".dd" => todo!(),
        ".include" => todo!(),
        ".section" => todo!(),
        _ => return Err(ParseError::UnexpectedInstructionMnemonic { span }),
    }

    Ok(())
}

fn gpr<'a>(tokens: &mut TokensIter<'a>) -> ParserResult<Gpr> {
    let (mnemonic, span) = identifier(tokens)?;
    mnemonic
        .strip_prefix("r")
        .and_then(|suffix| suffix.parse::<u16>().ok())
        .and_then(|n| Gpr::try_from(n).ok())
        .ok_or_else(|| ParseError::InvalidGprName { span })
}

fn cr<'a>(tokens: &mut TokensIter<'a>) -> ParserResult<ControlRegister> {
    let (mnemonic, span) = identifier(tokens)?;
    mnemonic
        .parse()
        .map_err(|_| ParseError::InvalidCrName { span })
}

/// Parse and evaluate an expression as an immediate value input to an instruction
/// and cast it to the proper type.
/// Uses a type Intermediate to do one extra conversion, to work around missing
/// conversions from too large type in uX.
fn immediate<'a, Intermediate: TryFrom<Value>, T: TryFrom<Intermediate>>(
    state: &mut ParseState,
    tokens: &mut TokensIter<'a>,
) -> ParserResult<T> {
    let (value, span) =
        expr::expression(tokens, &|symbol_name| state.get_symbol_value(symbol_name))?;
    value
        .try_into()
        .ok()
        .and_then(|x: Intermediate| x.try_into().ok())
        .ok_or_else(|| ParseError::ValueOutOfRange { span })
}

fn identifier<'a>(tokens: &mut TokensIter<'a>) -> ParserResult<(&'a str, Span)> {
    match tokens.next() {
        Some((Token::Identifier(identifier), span)) => Ok((identifier, span)),
        Some((_, span)) => Err(ParseError::UnexpectedToken {
            expected: Cow::Borrowed("identifier"),
            span,
        }),
        None => Err(ParseError::UnexpectedEof {
            expected: Cow::Borrowed("identifier"),
        }),
    }
}

fn one_token<'a>(tokens: &mut TokensIter<'a>, expected: Token<'a>) -> ParserResult<Span> {
    match tokens.next() {
        Some((t, span)) if t == expected => Ok(span),
        Some((_, span)) => Err(ParseError::UnexpectedToken {
            expected: Cow::Owned(format!("{expected:?}")),
            span,
        }),
        None => Err(ParseError::UnexpectedEof {
            expected: Cow::Owned(format!("{expected:?}")),
        }),
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("Unexpected token")]
    UnexpectedToken {
        expected: Cow<'static, str>,
        span: Span,
    },
    #[error("Unexpected end of file")]
    UnexpectedEof { expected: Cow<'static, str> },
    #[error("Unexpected instruction mnemonic")]
    UnexpectedInstructionMnemonic { span: Span },
    #[error("Invalid general purpose register name format")]
    InvalidGprName { span: Span },
    #[error("Invalid control register name format")]
    InvalidCrName { span: Span },
    #[error("Value out of range")]
    ValueOutOfRange { span: Span },
    #[error("Undefined symbol")]
    UndefinedSymbol { span: Span },
    #[error("Negative shift ammount")]
    NegativeShiftAmmount { span: Span },
    #[error("Redefinition of symbol")]
    SymbolRedefinition {
        span: Span,
        previous_definition: Span,
    },
    #[error("Symbol changed value in second pass")]
    SymbolChangedValue { span: Span },

    #[error("Other error: {description}")]
    OtherError { description: String },
}

/// This is a simple precedence climbing expression pareser/evaluator.
/// To make it easier to test, it plugs into the assembler state only through one function
/// `get_symbol`, that returns symbol value given its name, or None if the symbol is not defined.
mod expr {
    use std::borrow::Cow;

    use super::{one_token, ParseError, ParserResult, Span, Token, TokensIter};
    use crate::assembler::state::Value;

    pub fn expression<'a>(
        tokens: &mut TokensIter<'a>,
        get_symbol: &impl Fn(&str) -> Option<Value>,
    ) -> ParserResult<(Value, Span)> {
        let (first_value, first_span) = value(tokens, get_symbol)?;
        main(first_value, first_span, 0, tokens, get_symbol)
    }

    /// Parse and evaluate a single value that has lower precedence than all binary operators.
    /// Handles atoms (number / identifier), parenthesised expressions and unary operators.
    fn value<'a>(
        tokens: &mut TokensIter<'a>,
        get_symbol: &impl Fn(&str) -> Option<Value>,
    ) -> ParserResult<(Value, Span)> {
        static EXPECTED: &str = "`(`, `+`, `-`, `!`, `~`, identifier or number";
        match tokens.next() {
            Some((Token::Number(n), span)) => Ok((n, span)),
            Some((Token::Identifier(ident), span)) => {
                let v = get_symbol(ident)
                    .ok_or_else(|| ParseError::UndefinedSymbol { span: span.clone() })?;
                Ok((v, span))
            }
            Some((Token::LParen, span1)) => {
                let (v, _) = expression(tokens, get_symbol)?;
                let span2 = one_token(tokens, Token::RParen)?;
                Ok((v, span1.start..span2.end))
            }
            // Unary operators:
            Some((Token::Plus, span1)) => {
                let (v, span2) = value(tokens, get_symbol)?;
                Ok((v, span1.start..span2.end))
            }
            Some((Token::Minus, span1)) => {
                let (v, span2) = value(tokens, get_symbol)?;
                let result_span = span1.start..span2.end;
                let result = v.checked_neg().ok_or_else(|| ParseError::ValueOutOfRange {
                    span: result_span.clone(),
                })?;
                Ok((result, result_span))
            }
            Some((Token::Not, span1)) => {
                let (v, span2) = value(tokens, get_symbol)?;
                Ok(((v == 0).into(), span1.start..span2.end))
            }
            Some((Token::BitNot, span1)) => {
                let (v, span2) = value(tokens, get_symbol)?;
                Ok((!v, span1.start..span2.end))
            }
            Some((_, span)) => Err(ParseError::UnexpectedToken {
                expected: Cow::Borrowed(EXPECTED),
                span,
            }),
            None => Err(ParseError::UnexpectedEof {
                expected: Cow::Borrowed(EXPECTED),
            }),
        }
    }

    fn main<'a>(
        lhs: Value,
        lhs_span: Span,
        min_precedence: u32,
        tokens: &mut TokensIter<'a>,
        get_symbol: &impl Fn(&str) -> Option<Value>,
    ) -> ParserResult<(Value, Span)> {
        let mut lhs = lhs;
        let mut lhs_span = lhs_span;

        loop {
            tokens.reset_peek();
            let Some(op_precedence) = tokens.peek()
                .and_then(|(t, _span)| binary_operator_precedence(t))
                .and_then(|precedence| if precedence >= min_precedence { Some(precedence) } else { None })
            else {
                break;
            };
            let Some((op, _op_span)) = tokens.next() else { unreachable!(); };

            let (mut rhs, mut rhs_span) = value(tokens, get_symbol)?;
            loop {
                let Some(_) = tokens.peek()
                    .and_then(|(t, _span)| binary_operator_precedence(t))
                    .and_then(|precedence|
                        if precedence > op_precedence {
                            Some(precedence)
                        } else {
                            None
                        })
                else {
                    break;
                };
                (rhs, rhs_span) = main(rhs, rhs_span, op_precedence + 1, tokens, get_symbol)?;
                tokens.reset_peek();
            }

            (lhs, lhs_span) = eval_binary_operator(lhs, lhs_span, rhs, rhs_span, &op)?;
        }

        Ok((lhs, lhs_span))
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
        lhs_span: Span,
        rhs: Value,
        rhs_span: Span,
        operator: &Token,
    ) -> ParserResult<(Value, Span)> {
        fn from_bool(b: bool) -> Option<Value> {
            Some(b.into())
        }

        let v =
            match operator {
                Token::Asterisk => lhs.checked_mul(rhs),
                Token::Slash => lhs.checked_div(rhs),
                Token::Percent => lhs.checked_rem(rhs),
                Token::Plus => lhs.checked_add(rhs),
                Token::Minus => lhs.checked_sub(rhs),
                Token::Shl => lhs.checked_shl(rhs.try_into().map_err(|_| {
                    ParseError::NegativeShiftAmmount {
                        span: rhs_span.clone(),
                    }
                })?),
                Token::Shr => lhs.checked_shr(rhs.try_into().map_err(|_| {
                    ParseError::NegativeShiftAmmount {
                        span: rhs_span.clone(),
                    }
                })?),
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

        let span = lhs_span.start..rhs_span.end;

        match v {
            Some(v) => Ok((v, span)),
            None => Err(ParseError::ValueOutOfRange { span }),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::assembler::lexer::tokenize_str;
        use assert_matches::assert_matches;
        use test_case::test_case;
        use test_strategy::proptest;

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
            let expected_span = 0..input.len();
            let input = format!("{input}+");
            let mut tokens = tokenize_str(&input);
            let (result, span) = value(&mut tokens, &get_symbol).unwrap();
            assert_eq!(result, expected);
            assert_eq!(span, expected_span);
            assert_matches!(tokens.next(), Some((Token::Plus, _)));
        }

        #[test_case("" ; "eof")]
        #[test_case("+" ; "unexpected_token_1")]
        #[test_case(";" ; "unexpected_token_2")]
        #[test_case("9999999999999999999999" ; "error_token")]
        #[test_case("bar" ; "undefined_symbol")]
        #[test_case("(1" ; "missing_rparen")]
        #[test_case("()" ; "empty_parentheses")]
        fn value_error(input: &str) {
            let mut tokens = tokenize_str(input);
            let _err = value(&mut tokens, &no_symbols).unwrap_err();
            // TODO: Check error content
        }

        #[test_case("1+1", 2; "simple")]
        #[test_case("1+1 2+2", 2; "junk_after")]
        #[test_case("5*2/3", 3; "left_associativity")]
        #[test_case("5*2+3&0xfe", 12; "increasing_precedence")]
        #[test_case("1|2+3*4", 15; "decreasing_precedence")]
        #[test_case("(1 << 8) - 1", 255; "bitmask1")]
        #[test_case("0xabcd & ~((1 << 8) - 1)", 0xab00; "bitmask2")]
        #[test_case("2*3 - 4*5 + 6/3", -12; "mul_div_add_sub")]
        #[test_case("1 + 1 == 2 + 0", 1; "equals")]
        #[test_case("-2_147_483_647 - 1", -2_147_483_648; "minimum_value")]
        fn expression_happy_path(input: &str, expected: Value) {
            let mut tokens = tokenize_str(&input);
            let (result, _span) = expression(&mut tokens, &no_symbols).unwrap();
            assert_eq!(result, expected);
        }

        #[test_case("1+" ; "missing_rhs")]
        #[test_case("/1" ; "missing_lhs")]
        #[test_case("1/0" ; "divide_by_zero")]
        #[test_case("-(-2_147_483_647 - 1)" ; "neg_overflow")]
        #[test_case("1 << -2"; "shl_neg")]
        #[test_case("1 >> -2"; "shr_neg")]
        #[test_case("0xffffff * 0xffffff"; "mul_overflow")]
        fn expression_error(input: &str) {
            let mut tokens = tokenize_str(input);
            let _err = expression(&mut tokens, &no_symbols).unwrap_err();
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
        #[test_case("0b110010 & 0b101010", 0b100010; "and")]
        #[test_case("0b110010 | 0b101010", 0b111010; "or")]
        #[test_case("0b110010 ^ 0b101010", 0b011000; "xor")]
        fn evaluate_binary_operator_happy_path(input: &str, expected: Value) {
            let mut tokens = tokenize_str(&input);
            let (lhs, lhs_span) =
                assert_matches!(tokens.next(), Some((Token::Number(n), span)) => (n, span));
            let op = tokens.next().unwrap().0;
            let (rhs, rhs_span) =
                assert_matches!(tokens.next(), Some((Token::Number(n), span)) => (n, span));

            let (result, span) = eval_binary_operator(lhs, lhs_span, rhs, rhs_span, &op).unwrap();

            assert_eq!(result, expected);
            assert_eq!(span, 0..input.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
