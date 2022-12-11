use itertools::{Itertools, MultiPeek};
use logos::{Logos, SpannedIter};

pub use logos::Span;

use std::borrow::Cow;

pub type TokensIter<'a> = MultiPeek<SpannedIter<'a, Token<'a>>>;

/// Convert a string slice to a iterator of tokens with slice and with ability to peek
pub fn tokenize_str<'a>(s: &'a str) -> TokensIter<'a> {
    Token::lexer(s).spanned().multipeek()
}

#[derive(Debug)]
struct Tokenized<'a>(Vec<(Token<'a>, Span)>);

impl<'a> Tokenized<'a> {}

#[derive(Logos, Debug, PartialEq, Eq)]
pub enum Token<'a> {
    #[regex(r"[._a-zA-Z][._a-zA-Z0-9]*", |x| x.slice())]
    Identifier(&'a str),

    #[regex(r"[0-9][0-9_]*", parse_dec)]
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
    #[token("=")]
    Assign,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Asterisk,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("<<")]
    Shl,
    #[token(">>")]
    Shr,
    #[token("&&")]
    LogicalAnd,
    #[token("||")]
    LogicalOr,
    #[token("!")]
    Not,
    #[token("&")]
    BitAnd,
    #[token("|")]
    BitOr,
    #[token("^")]
    BitXor,
    #[token("~")]
    BitNot,
    #[token(";")]
    Semicolon,
    #[regex(r"(#[^\n]*)?\n", priority = 2)]
    Eol,

    #[error]
    #[regex(r"[\t\v\f\r ]", logos::skip)]
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
            .strip_prefix("0o")
            .or_else(|| slice.strip_prefix("0O"))
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
        r"\\u[{][0-9a-fA-F][0-9a-fA-F]?[0-9a-fA-F]?[0-9a-fA-F]?[0-9a-fA-F]?[0-9a-fA-F]?[}]",
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

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::multizip;
    use proptest::prelude::*;
    use test_case::test_case;
    use test_strategy::proptest;

    macro_rules! assert_tokens(
        ($s:expr, $($tokens:expr),*) => {
            use Token::*;
            assert_eq!(tokenize($s), [$($tokens),*]);
        }
    );

    #[test_case(r"abcd", &[Token::Identifier("abcd")]; "identifier_simple_1")]
    #[test_case(r".abcd1", &[Token::Identifier(".abcd1")]; "identifier_simple_2")]
    #[test_case(r"_123", &[Token::Identifier("_123")]; "identifier_numerical")]
    #[test_case(r"0x1_23", &[Token::Number(0x123)]; "number")]
    #[test_case(r"0x", &[Token::Error]; "num_only_prefix")]
    #[test_case(r"0xefg123", &[Token::Number(0xef), Token::Identifier("g123")]; "num_hex_different_character")]
    #[test_case(r"0b0123", &[Token::Number(0b01), Token::Number(23)]; "num_bin_different_character")]
    #[test_case(r"2_147_483_647", &[Token::Number(2_147_483_647)]; "num_max_value")]
    #[test_case(r"2_147_483_648", &[Token::Error]; "num_max_value_plus_one")]
    #[test_case(r#""abc""#, &[Token::String(Cow::Borrowed("abc"))]; "string_simple")]
    #[test_case(r#""\u{1f44d}""#, &[Token::String(Cow::Borrowed("👍"))]; "string_unicode_escape")]
    #[test_case(r#""\u{1F44D}""#, &[Token::String(Cow::Borrowed("👍"))]; "string_capitalized_unicode_escape")]
    #[test_case(r#""\\a\"b\nc\rd\te\0f\u{20}""#, &[Token::String(Cow::Borrowed("\\a\"b\nc\rd\te\0f "))]; "string_all_escapes")]
    #[test_case(r#""\q""#, &[Token::Error]; "string_invalid_escape")]
    #[test_case(r#""\u{aX}""#, &[Token::Error]; "string_invalid_unicode_escape_syntax1")]
    #[test_case(r#""\u{20""#, &[Token::Error]; "string_invalid_unicode_escape_syntax2")]
    #[test_case(r#""\u20""#, &[Token::Error]; "string_invalid_unicode_escape_syntax3")]
    #[test_case(r#""\u{0x110000}""#, &[Token::Error]; "string_invalid_unicode_escape_value")]
    #[test_case("\"abc\ndef\"", &[Token::Error, Token::Eol, Token::Identifier("def"), Token::Error]; "string_unescaped_newline")]
    #[test_case("123 + 456", &[Token::Number(123), Token::Plus, Token::Number(456)]; "addition_example")]
    #[test_case("abc\ndef;ghi", &[
        Token::Identifier("abc"),
        Token::Eol,
        Token::Identifier("def"),
        Token::Semicolon,
        Token::Identifier("ghi")
    ]; "eol_semicolon")]
    #[test_case("abc#comment\ndef", &[
        Token::Identifier("abc"),
        Token::Eol,
        Token::Identifier("def")
    ]; "eol_comment")]
    #[test_case("abc\n   	\n#comment\n  #comment\ndef", &[
        Token::Identifier("abc"),
        Token::Eol,
        Token::Eol,
        Token::Eol,
        Token::Eol,
        Token::Identifier("def")
    ]; "multiple_eol_with_whitespace_and_comments")]
    fn tokenize_examples(s: &str, expected: &[Token]) {
        assert_eq!(tokenize(s), expected);
    }

    #[proptest]
    fn num(#[strategy(valid_num_token_strategy())] value_s: (i32, String)) {
        let (value, s) = value_s;
        println!("{s:?}, {value}");
        assert_tokens!(&s, Number(value));
    }

    /// Test unicode escape that contains any string in the braces which
    /// is a valid string character and doesn't end the escape sequence
    #[proptest]
    fn string_invalid_unicode_escape_syntax4(
        #[strategy(r#""\\u[{][^a-zA-Z0-9\\"\x00-\x1F\x7F}]*[}]""#)] input: String,
    ) {
        assert_tokens!(&input, Error);
    }

    #[proptest]
    fn string_valid_unicode_escape(c: char) {
        let input = format!(r#""\u{{{:x}}}""#, u32::from(c));
        let mut expected = std::string::String::new();
        expected.push(c);
        assert_tokens!(&input, String(Cow::Owned(expected)));
    }

    /// Check that no input string crashes when lexing
    #[proptest]
    fn no_failures(input: String) {
        tokenize(&input);
    }

    #[proptest]
    fn comment(#[strategy(r"abc #[^\n]*\ndef")] input: String) {
        assert_tokens!(&input, Identifier("abc"), Eol, Identifier("def"));
    }

    fn tokenize(s: &str) -> Vec<Token> {
        Token::lexer(s).into_iter().collect()
    }

    /// Proptest strategy for producing a value and a valid numerical literal that
    /// evaluates to it.
    fn valid_num_token_strategy() -> impl Strategy<Value = (i32, String)> {
        (
            0u64..=(i32::MAX as u64),
            prop::sample::select([10u64, 16u64, 2u64, 8u64].as_slice()),
            0usize..5usize,
        )
            .prop_flat_map(|(value, base, leading_zeros)| {
                (
                    Just(value as i32),
                    num_token_strategy_p(value, base, leading_zeros),
                )
            })
    }

    fn num_token_strategy_p(
        value: u64,
        base: u64,
        leading_zeros: usize,
    ) -> impl Strategy<Value = String> {
        let len = DigitsIterator { value, base }.count() + 1;
        let padding_strategy =
            prop::collection::vec(0usize..=3usize, len + leading_zeros..=len + leading_zeros);
        let caps_strategy = prop::collection::vec(prop::bool::ANY, len..=len);
        (padding_strategy, caps_strategy).prop_map(move |(padding, caps)| {
            let mut ret = String::new();
            if base != 10 {
                ret.push('0');
                let base_marker = match base {
                    2 => 'b',
                    8 => 'o',
                    16 => 'x',
                    _ => unreachable!(),
                };
                ret.push(if caps[0] {
                    base_marker.to_ascii_uppercase()
                } else {
                    base_marker
                });
                for _ in 0..padding[0] {
                    ret.push('_');
                }
            }

            for pad in &padding[1..=leading_zeros] {
                ret.push('0');
                for _ in 0..*pad {
                    ret.push('_');
                }
            }

            if value == 0 {
                ret.push('0');
            } else {
                let digits: Vec<_> = DigitsIterator { value, base }.collect();
                for (digit, pad, cap) in multizip((
                    digits.iter().rev(),
                    &padding[leading_zeros + 1..],
                    &caps[1..],
                )) {
                    let digit = char::from_digit(*digit, base as u32).unwrap();
                    ret.push(if *cap {
                        digit.to_ascii_uppercase()
                    } else {
                        digit
                    });
                    for _ in 0..*pad {
                        ret.push('_');
                    }
                }
            }

            ret
        })
    }

    struct DigitsIterator {
        value: u64,
        base: u64,
    }

    impl Iterator for DigitsIterator {
        type Item = u32;
        fn next(&mut self) -> Option<u32> {
            if self.value == 0 {
                None
            } else {
                let ret = self.value % self.base;
                self.value /= self.base;
                Some(ret as u32)
            }
        }
    }
}
