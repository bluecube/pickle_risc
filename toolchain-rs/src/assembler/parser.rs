use crate::assembler::lexer::{Span, Token};
use crate::assembler::state::{ParseState, ScopeId};
use crate::instruction::{ControlRegister, Gpr, Instruction, Word};

use logos::SpannedIter;

use itertools::MultiPeek;

type TokensIter<'a> = MultiPeek<SpannedIter<'a, Token<'a>>>;
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
            state.define_symbol(id, state.current_pc, Some(scope_id))?;
            scope_rest(state, tokens, false)?;
        }
        _ => {
            state.define_symbol(id, state.current_pc, None)?;
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

fn immediate<'a, T>(state: &mut ParseState, tokens: &mut TokensIter<'a>) -> ParserResult<T> {
    todo!();
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
