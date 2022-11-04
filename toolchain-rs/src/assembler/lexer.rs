pub use logos::Logos;

use std::borrow::Cow;

#[derive(Logos, Debug, PartialEq, Eq)]
pub enum Token<'a> {
    #[regex(r"[._a-zA-Z][._a-zA-Z0-9]*", |x| x.slice())]
    Identifier(&'a str),

    #[regex(r"[1-9][0-9_]*|0", parse_dec)]
    #[regex(r"0[bB]_*[01][01_]*", parse_bin)]
    #[regex(r"0[oO]_*[0-7][0-7_]*", parse_oct)]
    #[regex(r"0[xX]_*[0-9a-fA-F][0-9a-fA-F_]*", parse_hex)]
    Number(i32),

    #[regex(r#""([^\\"\x00-\x1F\x7F]|\\[^\x00-\x1F\x7F])*""#, parse_string)]
    String(std::borrow::Cow<'a, str>),

    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("==")]
    Eq,
    #[token("!=")]
    Neq,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Asterisk,
    #[token("/")]
    Slash,
    #[token("<<")]
    Shl,
    #[token(">>")]
    Shr,
    #[token("**")]
    Pow,
    #[token("&&")]
    LogicalAnd,
    #[token("||")]
    LogicalOr,
    #[token("&")]
    BitAnd,
    #[token("|")]
    BitOr,

    #[regex(r"[\n;]", priority = 2)]
    Eol,

    #[error]
    #[regex(r"[[:space:]]+|#[^\n]*\n", logos::skip)]
    Error,
}

/// Parse a positive integer in a given base to i32, ignoring underscores.
/// Returns None if the value is out of range, panics if encountering unexpected character.
fn parse_num(s: &str, base: i32) -> Option<i32> {
    s.chars()
        .filter(|c| *c != '_')
        .map(|c| c.to_digit(base as u32).unwrap() as i32)
        .try_fold(0i32, |accumulator, digit| {
            accumulator
                .checked_mul(base)
                .and_then(|x| x.checked_add(digit))
        })
}

fn parse_dec<'a>(lex: &logos::Lexer<'a, Token<'a>>) -> Option<i32> {
    parse_num(lex.slice(), 10)
}

fn parse_bin<'a>(lex: &logos::Lexer<'a, Token<'a>>) -> Option<i32> {
    let slice = lex.slice();
    parse_num(
        slice
            .strip_prefix("0b")
            .or_else(|| slice.strip_prefix("0B"))
            .unwrap(),
        2,
    )
}

fn parse_hex<'a>(lex: &logos::Lexer<'a, Token<'a>>) -> Option<i32> {
    let slice = lex.slice();
    parse_num(
        slice
            .strip_prefix("0x")
            .or_else(|| slice.strip_prefix("0X"))
            .unwrap(),
        16,
    )
}

fn parse_oct<'a>(lex: &logos::Lexer<'a, Token<'a>>) -> Option<i32> {
    let slice = lex.slice();
    parse_num(
        slice
            .strip_prefix("0x")
            .or_else(|| slice.strip_prefix("0X"))
            .unwrap(),
        8,
    )
}

#[derive(Logos, Debug, PartialEq, Eq)]
enum StringToken<'a> {
    #[regex(r#"[^\\"\x00-\x1F\x7F]+"#)]
    Str(&'a str),

    #[token("\\\"")]
    EscQuote,
    #[token("\\\\")]
    EscBackslash,
    #[token("\\n")]
    EscNewline,
    #[token("\\r")]
    EscCr,
    #[token("\\t")]
    EscTab,
    #[token("\\0")]
    EscNull,
    #[regex(
        r"\\u[{][0-9a-zA-Z]?[0-9a-zA-Z]?[0-9a-zA-Z]?[0-9a-zA-Z]?[0-9a-zA-Z]?[0-9a-zA-Z]?[}]",
        parse_unicode_escape
    )]
    EscUnicode(char),

    #[error]
    Error,
}

impl<'a> StringToken<'a> {
    fn append_to(&self, s: &mut String) -> Option<()> {
        match self {
            StringToken::Str(value) => s.push_str(value),
            StringToken::EscQuote => s.push('"'),
            StringToken::EscBackslash => s.push('\\'),
            StringToken::EscNewline => s.push('\n'),
            StringToken::EscCr => s.push('\r'),
            StringToken::EscTab => s.push('\t'),
            StringToken::EscNull => s.push('\0'),
            StringToken::EscUnicode(c) => s.push(*c),
            StringToken::Error => return None,
        }

        Some(())
    }
}

fn parse_string<'a>(lex: &mut logos::Lexer<'a, Token<'a>>) -> Option<Cow<'a, str>> {
    let input = lex
        .slice()
        .strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .unwrap();

    let mut str_lex = StringToken::lexer(input);

    match str_lex.next() {
        None => Some(Cow::Borrowed(input)),
        Some(StringToken::Str(s)) if s.len() == input.len() => Some(Cow::Borrowed(input)),
        Some(first_token) => {
            let mut ret = String::with_capacity(input.len());
            first_token.append_to(&mut ret)?;
            for token in str_lex {
                token.append_to(&mut ret)?;
            }
            Some(Cow::Owned(ret))
        }
    }
}

/// Parse a escape sequence at the begining of the string, returns tuple with parsed
/// character and number of bytes to skip in the input.
fn parse_unicode_escape<'a>(lex: &logos::Lexer<'a, StringToken<'a>>) -> Option<char> {
    let input = lex
        .slice()
        .strip_prefix("\\u{")
        .and_then(|x| x.strip_suffix('}'))
        .unwrap();
    char::from_u32(u32::from_str_radix(input, 16).unwrap())
}
