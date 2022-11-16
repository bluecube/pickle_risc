pub use logos::Logos;

use std::borrow::Cow;

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
    use test_strategy::proptest;

    macro_rules! assert_tokens(
        ($s:expr, $($tokens:expr),*) => {
            use Token::*;
            assert_eq!(tokenize($s), [$($tokens),*]);
        }
    );

    #[test]
    fn test_identifier_simple1() {
        assert_tokens!(r"abcd", Identifier("abcd"));
    }

    #[test]
    fn test_identifier_simple2() {
        assert_tokens!(r".abcd1", Identifier(".abcd1"));
    }

    #[test]
    fn test_identifier_numerical() {
        assert_tokens!(r"_123", Identifier("_123"));
    }

    #[test]
    fn test_num_example() {
        assert_tokens!(r"0x1_23", Number(0x123));
    }

    #[proptest]
    fn test_num(#[strategy(valid_num_token_strategy())] value_s: (i32, String)) {
        let (value, s) = value_s;
        println!("{s:?}, {value}");
        assert_tokens!(&s, Number(value));
    }

    #[test]
    fn test_num_only_prefix() {
        assert_tokens!(r"0x", Error);
    }

    #[test]
    fn test_num_hex_different_character() {
        assert_tokens!(r"0xefg123", Number(0xef), Identifier("g123"));
    }

    #[test]
    fn test_num_bin_different_character() {
        assert_tokens!(r"0b0123", Number(0b01), Number(23));
    }

    #[test]
    fn test_simple_string() {
        assert_tokens!(r#""abc""#, String(Cow::Borrowed("abc")));
    }

    #[test]
    fn test_string_unicode_escape() {
        assert_tokens!(r#""\u{1f44d}""#, String(Cow::Borrowed("👍")));
    }

    #[test]
    fn test_string_capitalized_unicode_escape() {
        assert_tokens!(r#""\u{1F44D}""#, String(Cow::Borrowed("👍")));
    }

    #[test]
    fn test_string_all_escapes() {
        assert_tokens!(
            r#""\\a\"b\nc\rd\te\0f\u{20}""#,
            String(Cow::Borrowed("\\a\"b\nc\rd\te\0f "))
        );
    }

    #[test]
    fn test_string_invalid_escape() {
        assert_tokens!(r#""\q""#, Error);
    }

    #[test]
    fn test_string_invalid_unicode_escape_syntax1() {
        assert_tokens!(r#""\u{aX}""#, Error);
    }

    #[test]
    fn test_string_invalid_unicode_escape_syntax2() {
        assert_tokens!(r#""\u{20""#, Error);
    }

    #[test]
    fn test_string_invalid_unicode_escape_syntax3() {
        assert_tokens!(r#""\u20""#, Error);
    }

    /// Test unicode escape that contains any string in the braces which
    /// is a valid string character and doesn't end the escape sequence
    #[proptest]
    fn test_string_invalid_unicode_escape_syntax4(
        #[strategy(r#""\\u[{][^a-zA-Z0-9\\"\x00-\x1F\x7F}]*[}]""#)] input: String,
    ) {
        assert_tokens!(&input, Error);
    }

    #[test]
    fn test_string_invalid_unicode_escape_value() {
        assert_tokens!(r#""\u{0x110000}""#, Error);
    }

    #[test]
    fn test_string_unescaped_newline() {
        assert_tokens!("\"abc\ndef\"", Error, Eol, Identifier("def"), Error);
    }

    #[proptest]
    fn test_string_valid_unicode_escape(c: char) {
        let input = format!(r#""\u{{{:x}}}""#, u32::from(c));
        let mut expected = std::string::String::new();
        expected.push(c);
        assert_tokens!(&input, String(Cow::Owned(expected)));
    }

    /// Check that no input string crashes when lexing
    #[proptest]
    fn test_no_failures(input: String) {
        tokenize(&input);
    }

    #[test]
    fn test_addition_example() {
        assert_tokens!("123 + 456", Number(123), Plus, Number(456));
    }

    #[test]
    fn test_asterisks() {
        assert_tokens!("*****", Pow, Pow, Asterisk);
    }

    #[test]
    fn test_eol_semicolon() {
        assert_tokens!(
            "abc\ndef;ghi",
            Identifier("abc"),
            Eol,
            Identifier("def"),
            Semicolon,
            Identifier("ghi")
        );
    }

    #[test]
    fn test_eol_comment() {
        assert_tokens!(
            "abc#comment\ndef",
            Identifier("abc"),
            Eol,
            Identifier("def")
        );
    }

    #[test]
    fn test_multiple_eol_with_whitespace_and_comments() {
        assert_tokens!(
            "abc\n   	\n#comment\n  #comment\ndef",
            Identifier("abc"),
            Eol,
            Eol,
            Eol,
            Eol,
            Identifier("def")
        );
    }

    #[proptest]
    fn test_comment(#[strategy(r"abc #[^\n]*\ndef")] input: String) {
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
