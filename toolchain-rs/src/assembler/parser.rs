use std::borrow::Cow;
use std::str::FromStr;

use mockall_double::double;

use crate::assembler::{
    expr_parser::expression,
    lexer::Token,
    files::{FileTokens, Location},
    ScopeId, Symbol, Value, AsmError, AsmResult,
};
#[double]
use crate::assembler::AssemblerState;
use crate::instruction::{ControlRegister, Gpr, Instruction};

pub(super) type ParseResult<'a, T> = Result<(T, Location, FileTokens<'a>), AsmError>;
pub(super) type ParseResultNoValue<'a> = Result<(Location, FileTokens<'a>), AsmError>;

pub(super) fn map_parse_result<'a, T, U>(result: ParseResult<'a, T>, f: impl FnOnce(&T, Location) -> AsmResult<U>) -> ParseResult<'a, U> {
    let (v, location, tokens) = result?;
    let mapped = f(&v, location)?;
    Ok((mapped, location, tokens))
}

pub fn file<'a>(state: &mut AssemblerState, tokens: FileTokens<'a>) -> AsmResult<()> {
    scope_content(state, tokens)?;

    match tokens.first() {
        Some((_, location)) => Err(AsmError::UnexpectedToken {
            expected: Cow::Borrowed("`{`, `;`, `\\n`, identifier or end of file"),
            location,
        }),
        None => Ok(()),
    }
}

/// Parses a label or a labeled scope
fn label<'a>(state: &mut AssemblerState, tokens: FileTokens<'a>) -> ParseResult<'a, ()> {
    let (id, id_location, tokens) = identifier(tokens)?;
    let (_, colon_location, tokens) = one_token(tokens, &Token::Colon)?;

    let definition_location = id_location.extend_to(&colon_location);

    match tokens.first() {
        Some((Token::LBrace, _)) => {
            let (scope_id, _, tokens) = scope_start(state, tokens)?;
            state.define_symbol(id, state.get_current_pc_symbol(Some(scope_id), definition_location))?;
            scope_content(state, tokens)?;
            let (_, scope_end_location, rest) = scope_end(state, tokens)?;
            Ok(((), definition_location.extend_to(&scope_end_location), rest))
        }
        _ => {
            state.define_symbol(id, state.get_current_pc_symbol(None, definition_location))?;
            Ok(((), definition_location, tokens))
        }
    }
}

fn assignment<'a>(state: &mut AssemblerState, tokens: FileTokens<'a>) -> ParseResult<'a, ()> {
    let (id, id_location, tokens) = identifier(tokens)?;
    let ((), _, tokens) = one_token(tokens, &Token::Assign)?;

    let (value, value_location, rest) =
        expression(tokens, &|symbol_name| state.get_symbol_value(symbol_name))?;

    let definition_location = id_location.extend_to(&value_location);

    state.define_symbol(
        id,
        Symbol::Free {
            value,
            defined_at: definition_location,
        },
    )?;

    Ok(((), definition_location, rest))
}

fn anonymous_scope<'a>(
    state: &mut AssemblerState,
    tokens: FileTokens<'a>,
) -> ParseResult<'a, ()> {
    let (_, start_location, tokens) = scope_start(state, tokens)?;
    let (_, _, tokens) = scope_content(state, tokens)?;
    let (_, end_location, tokens) = scope_end(state, tokens)?;
    Ok(((), start_location.extend_to(&end_location), tokens))
}

fn scope_start<'a>(
    state: &mut AssemblerState,
    tokens: FileTokens<'a>,
) -> ParseResult<'a, ScopeId> {
    map_parse_result(
        one_token(tokens, &Token::LBrace),
        |_, _| Ok(state.push_scope())
    )
}

fn scope_content<'a>(state: &mut AssemblerState, tokens: FileTokens<'a>) -> ParseResult<'a, ()> {
    let mut tokens = tokens;
    let mut location = match tokens.first() {
        Some((_, location)) => location,
        None => tokens.empty_location(),
    };

    loop {
        let mut new_location = location;
        match tokens.first() {
            Some((Token::Identifier(ident), _)) => match tokens.rest().first() {
                Some((Token::Colon, _)) => {
                    (_, new_location, tokens) = label(state, tokens)?
                },
                Some((Token::Assign, _)) => {
                    (_, new_location, tokens) = assignment(state, tokens)?
                },
                _ if ident.starts_with('.') => {
                    (_, new_location, tokens) = pseudo_instruction(state, tokens)?
                },
                _ => {
                    (_, new_location, tokens) = instruction(state, tokens)?
                },
            },
            Some((Token::LBrace, _)) => {
                (_, new_location, tokens) = anonymous_scope(state, tokens)?
            },
            Some((Token::Eol, l)) | Some((Token::Semicolon, l)) => {
                // Skip empty lines
                new_location = l;
                tokens = tokens.rest();
            },
            _ => return Ok(((), location, tokens)),
        }

        location = location.extend_to(&new_location);
    }
}

fn scope_end<'a>(state: &mut AssemblerState, tokens: FileTokens<'a>) -> ParseResult<'a, ()> {
    let (_, location, rest) = one_token(tokens, &Token::RBrace)?;
    state.pop_scope();
    Ok(((), location, rest))
}

fn instruction<'a>(state: &mut AssemblerState, tokens: FileTokens<'a>) -> ParseResult<'a, ()> {
    use ux::*;
    let (mnemonic, mnemonic_location, tokens) = identifier(tokens)?;
    let (instruction, location, rest) = include!(concat!(env!("OUT_DIR"), "/parse_asm_match.rs"))
        .ok_or_else(|| AsmError::UnexpectedInstructionMnemonic { location: mnemonic_location })?;
    state.emit_word(instruction.into());
    Ok(((), location, rest))
}

fn pseudo_instruction<'a>(
    state: &mut AssemblerState,
    tokens: FileTokens<'a>,
) -> ParseResult<'a, ()> {
    let (mnemonic, location, tokens) = identifier(tokens)?;
    match mnemonic {
        ".db" => todo!(),
        ".dw" => todo!(),
        ".dd" => todo!(),
        ".include" => todo!(),
        ".section" => todo!(),
        _ => return Err(AsmError::UnexpectedInstructionMnemonic { location }),
    }

    Ok(((), location, tokens))
}

fn gpr<'a>(tokens: FileTokens<'a>) -> ParseResult<Gpr> {
    map_parse_result(
        identifier(tokens),
        |ident, location| Gpr::from_str(ident).map_err(|_| AsmError::InvalidGprName { location })
    )
}

fn cr<'a>(tokens: FileTokens<'a>) -> ParseResult<ControlRegister> {
    map_parse_result(
        identifier(tokens),
        |ident, location| ControlRegister::from_str(ident)
            .map_err(|_| AsmError::InvalidCrName { location })
    )
}

/// Parse and evaluate an expression as an immediate value input to an instruction
/// and cast it to the proper type.
/// Uses a type Intermediate to do one extra conversion, to work around missing
/// conversions from too large type in uX.
fn immediate<'a, Intermediate: TryFrom<Value>, T: TryFrom<Intermediate>>(
    state: &mut AssemblerState,
    tokens: FileTokens<'a>,
) -> ParseResult<'a, T> {
    map_parse_result(
        expression(tokens, &|symbol_name| state.get_symbol_value(symbol_name)),
        |value, location| Intermediate::try_from(*value).ok()
            .and_then(|x| T::try_from(x).ok())
            .ok_or_else(|| AsmError::ValueOutOfRange { location })
    )
}

fn identifier<'a>(tokens: FileTokens<'a>) -> ParseResult<&'a str> {
    match tokens.first() {
        Some((Token::Identifier(identifier), location)) => Ok((&identifier, location, tokens.rest())),
        Some((_, location)) => Err(AsmError::UnexpectedToken {
            expected: Cow::Borrowed("identifier"),
            location,
        }),
        None => Err(AsmError::UnexpectedEof {
            expected: Cow::Borrowed("identifier"),
        }),
    }
}

pub(super) fn one_token<'a>(tokens: FileTokens<'a>, expected: &Token) -> ParseResult<'a, ()> {
    match tokens.first() {
        Some((t, location)) if t == expected => Ok(((), location, tokens.rest())),
        Some((_, location)) => Err(AsmError::UnexpectedToken {
            expected: Cow::Owned(format!("{expected:?}")),
            location,
        }),
        None => Err(AsmError::UnexpectedEof {
            expected: Cow::Owned(format!("{expected:?}")),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembler::{
        files::SnippetTokenizer,
        Section
    };

    #[test]
    fn assignment_simple() {
        let tokenizer = SnippetTokenizer::new("abc = 123".to_owned());
        let mut mock = AssemblerState::default();
        let defined_at = tokenizer.location(0, 9);
        mock.expect_define_symbol()
            .withf(move |sym_name, symbol| {
                assert_eq!(sym_name, "abc");
                assert_eq!(
                    symbol,
                    &Symbol::Free {
                        value: 123,
                        defined_at
                    }
                );
                true
            })
            .return_const(AsmResult::Ok(()))
            .times(1);

        assignment(&mut mock, tokenizer.tokens()).unwrap();
    }

    #[test]
    fn assignment_expression() {
        let tokenizer = SnippetTokenizer::new("abc = def * 7".to_owned());
        let mut mock = AssemblerState::default();
        let defined_at = tokenizer.location(0, 13);
        mock.expect_define_symbol()
            .withf(move |sym_name, symbol| {
                assert_eq!(sym_name, "abc");
                assert_eq!(
                    symbol,
                    &Symbol::Free {
                        value: 21,
                        defined_at
                    }
                );
                true
            })
            .return_const(AsmResult::Ok(()))
            .times(1);
        mock.expect_get_symbol_value()
            .withf(|sym_name| {
                assert_eq!(sym_name, "def");
                true
            })
            .return_const(Some(3))
            .times(1);

        assignment(&mut mock, tokenizer.tokens()).unwrap();
    }

    #[test]
    fn label_simple() {
        let tokenizer = SnippetTokenizer::new("abc:".to_owned());
        let mut mock = AssemblerState::default();

        let mut sections = id_arena::Arena::<Section>::new();
        let section = sections.alloc(Section::default());

        let defined_at = tokenizer.location(0, 4);

        let symbol = Symbol::Location {
            section: section.clone(),
            offset: 0,
            attached_scope: None,
            defined_at,
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
                        defined_at,
                    }
                );
                true
            })
            .return_const(AsmResult::Ok(()))
            .times(1);
        mock.expect_get_current_pc_symbol()
            .return_const(symbol.clone())
            .withf(move |attached_scope, span| {
                assert_eq!(attached_scope, &None);
                assert_eq!(span, &symbol.get_defined_at());
                true
            })
            .times(1);

        label(&mut mock, tokenizer.tokens()).unwrap();
    }
}
