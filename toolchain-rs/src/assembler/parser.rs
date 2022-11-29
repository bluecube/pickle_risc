use crate::assembler::lexer::{Span, Token, TokensIter};
use crate::assembler::state::{ParseState, ScopeId, Value};
use crate::instruction::{ControlRegister, Gpr, Instruction};

type ParserResult<V> = Result<V, String>; // TODO: use codespan

pub fn top<'a>(state: &mut ParseState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    scope_rest(state, tokens, /* top_level */ true)
}

/// Parses a label or a labeled scope
fn label<'a>(state: &mut ParseState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    let (id, _) = identifier(tokens)?;
    one_token(tokens, Token::Colon)?;

    match tokens.peek() {
        Some((Token::LBrace, _)) => {
            let scope_id = scope_start(state, tokens)?;
            state.define_symbol(id, state.current_pc.into(), Some(scope_id))?;
            scope_rest(state, tokens, false)?;
        }
        _ => {
            state.define_symbol(id, state.current_pc.into(), None)?;
        }
    }

    Ok(())
}

fn anonymous_scope<'a>(state: &mut ParseState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    scope_start(state, tokens)?;
    scope_rest(state, tokens, false)
}

fn scope_start<'a>(state: &mut ParseState, tokens: &mut TokensIter<'a>) -> ParserResult<ScopeId> {
    one_token(tokens, Token::LBrace)?;
    Ok(state.push_scope())
}

fn scope_rest<'a>(
    state: &mut ParseState,
    tokens: &mut TokensIter<'a>,
    top_level: bool,
) -> ParserResult<()> {
    loop {
        tokens.reset_peek();
        match tokens.peek() {
            None if top_level => return Ok(()), // End of file
            Some((Token::RBrace, _)) if !top_level => return scope_end(state, tokens),
            Some(&(Token::Identifier(ident), _)) => match tokens.peek() {
                // Peek second token!
                Some((Token::Colon, _)) => label(state, tokens)?,
                _ if ident.starts_with('.') => pseudo_instruction(state, tokens)?,
                _ => instruction(state, tokens)?,
            },
            Some((Token::LBrace, _)) => anonymous_scope(state, tokens)?,
            Some((Token::Eol, _)) | Some((Token::Semicolon, _)) => continue, // Skip empty lines
            Some((Token::Error, _)) => return Err("Error parsing".to_owned()),
            _ => return Err("Unexpected token".to_owned()),
        }
    }
}

fn scope_end<'a>(state: &mut ParseState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    one_token(tokens, Token::RBrace)?;
    state.pop_scope();
    Ok(())
}

fn instruction<'a>(state: &mut ParseState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    use ux::*;
    let (mnemonic, _span) = identifier(tokens)?;
    let instruction = include!(concat!(env!("OUT_DIR"), "/parse_asm_match.rs"))
        .ok_or_else(|| "Unexpected instruction mnemonic".to_owned())?;
    todo!();
    Ok(())
}

fn pseudo_instruction<'a>(state: &mut ParseState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    let (mnemonic, _) = identifier(tokens)?;
    match mnemonic {
        ".db" => todo!(),
        ".dw" => todo!(),
        ".dd" => todo!(),
        ".include" => todo!(),
        ".section" => todo!(),
        _ => return Err("Unexpected pseudo instruction".into()),
    }

    Ok(())
}

fn gpr<'a>(tokens: &mut TokensIter<'a>) -> ParserResult<Gpr> {
    let (mnemonic, _) = identifier(tokens)?;
    mnemonic
        .strip_prefix("r")
        .and_then(|suffix| suffix.parse::<u16>().ok())
        .and_then(|n| Gpr::try_from(n).ok())
        .ok_or_else(|| "Bad format".to_owned())
}

fn cr<'a>(tokens: &mut TokensIter<'a>) -> ParserResult<ControlRegister> {
    let (mnemonic, _) = identifier(tokens)?;
    mnemonic
        .parse()
        .map_err(|_| "Unexpected control register name".to_owned())
}

fn immediate<'a, Intermediate: TryFrom<Value>, T: TryFrom<Intermediate>>(
    state: &mut ParseState,
    tokens: &mut TokensIter<'a>,
) -> ParserResult<T> {
    let value = expr::expression(tokens, &|symbol_name| state.get_symbol_value(symbol_name))?;
    value
        .try_into()
        .ok()
        .and_then(|x: Intermediate| x.try_into().ok())
        .ok_or_else(|| "Value out of range".to_owned())
}

fn identifier<'a>(tokens: &mut TokensIter<'a>) -> ParserResult<(&'a str, Span)> {
    match tokens.next() {
        Some((Token::Identifier(identifier), span)) => Ok((identifier, span)),
        _ => Err("Unexpected token".to_owned()),
    }
}

fn any_token<'a>(tokens: &mut TokensIter<'a>, expected: &[Token]) -> ParserResult<()> {
    match tokens.next() {
        Some((token, _span)) => {
            if expected
                .iter()
                .position(|expected_token| expected_token == &token)
                .is_some()
            {
                Ok(())
            } else {
                Err("Unexpected token".to_owned())
            }
        }
        None => Err("Unexpected EOF".to_owned()),
    }
}

fn one_token<'a>(tokens: &mut TokensIter<'a>, expected: Token) -> ParserResult<()> {
    any_token(tokens, std::slice::from_ref(&expected))
}

/// This is a simple precedence climbing expression pareser/evaluator.
/// To make it easier to test, it plugs into the assembler state only through one function
/// `get_symbol`, that returns symbol value given its name, or None if the symbol is not defined.
mod expr {
    use super::{one_token, ParserResult, Token, TokensIter};
    use crate::assembler::state::Value;

    pub fn expression<'a>(
        tokens: &mut TokensIter<'a>,
        get_symbol: &impl Fn(&str) -> Option<Value>,
    ) -> ParserResult<Value> {
        let v = value(tokens, get_symbol)?;
        main(v, 0, tokens, get_symbol)
    }

    /// Parse and evaluate a single value that has lower precedence than all binary operators.
    /// Handles atoms (number / identifier), parenthesised expressions and unary operators.
    fn value<'a>(
        tokens: &mut TokensIter<'a>,
        get_symbol: &impl Fn(&str) -> Option<Value>,
    ) -> ParserResult<Value> {
        match tokens.next() {
            Some((Token::Number(n), _)) => Ok(n),
            Some((Token::Identifier(ident), _span)) => {
                get_symbol(ident).ok_or_else(|| "Undefined identifier".to_owned())
            }
            Some((Token::LParen, _)) => {
                let v = expression(tokens, get_symbol)?;
                one_token(tokens, Token::RParen)?;
                Ok(v)
            }
            // Unary operators:
            Some((Token::Plus, _)) => value(tokens, get_symbol),
            Some((Token::Minus, _)) => Ok(-value(tokens, get_symbol)?), // Token value is always positive, this cannot overflow
            Some((Token::Not, _)) => {
                let v = value(tokens, get_symbol)?;
                Ok((v == 0).into())
            }
            Some((Token::BitNot, _)) => Ok(!value(tokens, get_symbol)?),

            Some(_) => Err("Unepected token".to_owned()),
            _ => Err("Unepected EOF".to_owned()),
        }
    }

    fn main<'a>(
        lhs: Value,
        min_precedence: u32,
        tokens: &mut TokensIter<'a>,
        get_symbol: &impl Fn(&str) -> Option<Value>,
    ) -> ParserResult<Value> {
        tokens.reset_peek();
        let mut lhs = lhs;

        loop {
            let Some(op_precedence) = tokens.peek()
                .and_then(|(t, _span)| binary_operator_precedence(t))
                .and_then(|precedence| if precedence >= min_precedence { Some(precedence) } else { None })
            else {
                break;
            };
            let Some((op, span)) = tokens.next() else { unreachable!(); };

            let mut rhs = value(tokens, get_symbol)?;
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
                rhs = main(rhs, op_precedence + 1, tokens, get_symbol)?;
                tokens.reset_peek();
            }

            lhs = eval_binary_operator(lhs, rhs, &op, span)?;
        }

        Ok(lhs)
    }

    /// Return binary operator precedence for a token, or None if the token doesn't
    /// correspond to a binary operator.
    /// Loosely based on C operator precedence table.
    fn binary_operator_precedence(token: &Token) -> Option<u32> {
        use Token::*;
        match token {
            Asterisk | Slash | Percent => Some(0),
            Plus | Minus => Some(1),
            Shl | Shr => Some(2),
            Eq | Neq | Lt | Gt | Le | Ge => Some(3),
            BitAnd => Some(4),
            BitXor => Some(5),
            BitOr => Some(6),
            _ => None,
        }
    }

    fn eval_binary_operator(
        lhs: Value,
        rhs: Value,
        operator: &Token,
        span: core::ops::Range<usize>,
    ) -> ParserResult<Value> {
        fn from_bool(b: bool) -> Option<Value> {
            Some(b.into())
        }

        match operator {
            Token::Asterisk => lhs.checked_mul(rhs),
            Token::Slash => lhs.checked_div(rhs),
            Token::Percent => lhs.checked_rem(rhs),
            Token::Plus => lhs.checked_add(rhs),
            Token::Minus => lhs.checked_sub(rhs),
            Token::Shl => lhs.checked_shl(
                rhs.try_into()
                    .map_err(|_| "Negative shift amount".to_owned())?,
            ),
            Token::Shr => lhs.checked_shr(
                rhs.try_into()
                    .map_err(|_| "Negative shift amount".to_owned())?,
            ),
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
        }
        .ok_or_else(|| "Value out of range".to_owned())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::assembler::lexer::tokenize_str;
        use test_strategy::proptest;
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
        #[test_case("(3 + 2))", 5 ; "parenthesis_expression")]
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
            let input = format!("{input}+");
            let mut tokens = tokenize_str(&input);
            let result = value(&mut tokens, &get_symbol).unwrap();
            assert_eq!(result, expected);
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

}
