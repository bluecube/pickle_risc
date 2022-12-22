use std::borrow::Cow;
use std::str::FromStr;

use mockall_double::double;
use thiserror::Error;

use crate::assembler::expr_parser::expression;
use crate::assembler::lexer::{Span, Token, TokensIter};
#[double]
use crate::assembler::state::AssemblerState;
use crate::assembler::state::{ScopeId, Symbol, Value};
use crate::instruction::{ControlRegister, Gpr, Instruction};

pub type ParserResult<V> = Result<V, ParseError>;

pub fn top<'a>(state: &mut AssemblerState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    scope_content(state, tokens)?;

    match tokens.next() {
        Some((_, span)) => Err(ParseError::UnexpectedToken {
            expected: Cow::Borrowed("`{`, `;`, `\\n`, identifier or end of file"),
            span,
        }),
        None => Ok(()),
    }
}

/// Parses a label or a labeled scope
fn label<'a>(state: &mut AssemblerState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    let (id, id_span) = identifier(tokens)?;
    let colon_span = one_token(tokens, Token::Colon)?;

    let span = id_span.start..colon_span.end;

    match tokens.peek() {
        Some((Token::LBrace, _)) => {
            let scope_id = scope_start(state, tokens)?;
            state.define_symbol(id, state.get_current_pc_symbol(Some(scope_id), span))?;
            scope_content(state, tokens)?;
            scope_end(state, tokens)?;
        }
        _ => {
            state.define_symbol(id, state.get_current_pc_symbol(None, span))?;
        }
    }

    Ok(())
}

fn assignment<'a>(state: &mut AssemblerState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    let (id, id_span) = identifier(tokens)?;
    one_token(tokens, Token::Assign)?;

    let (value, value_span) =
        expression(tokens, &|symbol_name| state.get_symbol_value(symbol_name))?;

    let span = id_span.start..value_span.end;

    state.define_symbol(
        id,
        Symbol::Free {
            value,
            defined_at: span,
        },
    )?;

    Ok(())
}

fn anonymous_scope<'a>(
    state: &mut AssemblerState,
    tokens: &mut TokensIter<'a>,
) -> ParserResult<()> {
    scope_start(state, tokens)?;
    scope_content(state, tokens)?;
    scope_end(state, tokens)?;
    Ok(())
}

fn scope_start<'a>(
    state: &mut AssemblerState,
    tokens: &mut TokensIter<'a>,
) -> ParserResult<ScopeId> {
    let _span = one_token(tokens, Token::LBrace)?;
    Ok(state.push_scope())
}

fn scope_content<'a>(state: &mut AssemblerState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
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

fn scope_end<'a>(state: &mut AssemblerState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    state.pop_scope();
    one_token(tokens, Token::RBrace)?;
    Ok(())
}

fn instruction<'a>(state: &mut AssemblerState, tokens: &mut TokensIter<'a>) -> ParserResult<()> {
    use ux::*;
    let (mnemonic, span) = identifier(tokens)?;
    let instruction = include!(concat!(env!("OUT_DIR"), "/parse_asm_match.rs"))
        .ok_or_else(|| ParseError::UnexpectedInstructionMnemonic { span })?;
    state.emit_word(instruction.into());
    Ok(())
}

fn pseudo_instruction<'a>(
    state: &mut AssemblerState,
    tokens: &mut TokensIter<'a>,
) -> ParserResult<()> {
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
    Gpr::from_str(mnemonic).map_err(|_| ParseError::InvalidGprName { span })
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
    state: &mut AssemblerState,
    tokens: &mut TokensIter<'a>,
) -> ParserResult<T> {
    let (value, span) = expression(tokens, &|symbol_name| state.get_symbol_value(symbol_name))?;
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

pub(super) fn one_token<'a>(
    tokens: &mut TokensIter<'a>,
    expected: Token<'a>,
) -> ParserResult<Span> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembler::lexer::tokenize_str;
    use crate::assembler::state::Section;

    #[test]
    fn assignment_simple() {
        let mut tokens = tokenize_str("abc = 123");
        let mut mock = AssemblerState::default();
        mock.expect_define_symbol()
            .withf(|sym_name, symbol| {
                assert_eq!(sym_name, "abc");
                assert_eq!(
                    symbol,
                    &Symbol::Free {
                        value: 123,
                        defined_at: 0..9
                    }
                );
                true
            })
            .return_const(Ok(()))
            .times(1);

        assignment(&mut mock, &mut tokens).unwrap();
    }

    #[test]
    fn assignment_expression() {
        let mut tokens = tokenize_str("abc = def * 7");
        let mut mock = AssemblerState::default();
        mock.expect_define_symbol()
            .withf(|sym_name, symbol| {
                assert_eq!(sym_name, "abc");
                assert_eq!(
                    symbol,
                    &Symbol::Free {
                        value: 21,
                        defined_at: 0..13
                    }
                );
                true
            })
            .return_const(Ok(()))
            .times(1);
        mock.expect_get_symbol_value()
            .withf(|sym_name| {
                assert_eq!(sym_name, "def");
                true
            })
            .return_const(Some(3))
            .times(1);

        assignment(&mut mock, &mut tokens).unwrap();
    }

    #[test]
    fn label_simple() {
        let mut tokens = tokenize_str("abc:");
        let mut mock = AssemblerState::default();

        let mut sections = id_arena::Arena::<Section>::new();
        let section = sections.alloc(Section::default());

        let symbol = Symbol::Location {
            section: section.clone(),
            offset: 0,
            attached_scope: None,
            defined_at: 0..4,
        };

        mock.expect_define_symbol()
            .withf(move |sym_name, symbol| {
                assert_eq!(sym_name, "abc");
                assert_eq!(
                    symbol,
                    &Symbol::Location {
                        section,
                        offset: 0,
                        attached_scope: None,
                        defined_at: 0..4
                    }
                );
                true
            })
            .return_const(Ok(()))
            .times(1);
        mock.expect_get_current_pc_symbol()
            .return_const(symbol.clone())
            .withf(move |attached_scope, span| {
                assert_eq!(attached_scope, &None);
                assert_eq!(span, &symbol.get_defined_at());
                true
            })
            .times(1);

        label(&mut mock, &mut tokens).unwrap();
    }
}
