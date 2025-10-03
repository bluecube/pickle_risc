use chumsky::{input::StrInput, prelude::*};

use crate::types::{AssemblerError, FileId, Span, Spanned};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token<'src> {
    Identifier(&'src str),
    MacroCall(&'src str),
    Number(i64),

    // Keywords
    Const,
    Macro,

    // Symbols
    Colon,
    Comma,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Lt,
    Gt,
    DoubleEqual,
    Neq,
    Le,
    Ge,
    Equal,
    Plus,
    Minus,
    Asterisk,
    Slash,
    Percent,
    DoubleLt,
    DoubleGt,
    DoubleAmpersand,
    DoublePipe,
    Exclamation,
    Ampersand,
    Pipe,
    Caret,
    Tilde,
    Dot,

    Eol,
}

pub fn tokenize<'src>(
    input: &'src str,
    file_id: Option<FileId>,
    errors: &mut Vec<AssemblerError>,
) -> Option<Vec<Spanned<Token<'src>>>> {
    let input = input.map_span(move |x| Span {
        file_id,
        start: x.start,
        end: x.end,
    });
    let (tokens, tokenize_errors) = lexer().parse(input).into_output_errors();
    errors.extend(
        tokenize_errors
            .into_iter()
            .map(|e| AssemblerError::InvalidToken(e.into())),
    );
    tokens
}

fn lexer<'src, I>()
-> impl Parser<'src, I, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char, Span>>>
where
    I: StrInput<'src, Span = Span, Token = char, Slice = &'src str>,
{
    let keyword = choice((
        just("const").to(Token::Const),
        just("macro").to(Token::Macro),
    ))
    .labelled("keyword");
    let identifier = text::ascii::ident()
        .then(just('!').or_not())
        .map(|(name, bang)| {
            if bang.is_some() {
                Token::MacroCall(name)
            } else {
                Token::Identifier(name)
            }
        })
        .labelled("identifier");

    // Numbers could technically start with a character, but since identifiers are parsed earlier,
    // we can never get to that.
    let number = choice((just("0x").to(16), just("0b").to(2), empty().to(10)))
        .then(text::digits(36).at_least(1).to_slice())
        .try_map(|(base, s), span| match i64::from_str_radix(s, base) {
            Ok(v) => Ok(Token::Number(v)),
            Err(_) => Err(Rich::custom(span, "invalid digit")),
        })
        .labelled("number");

    let symbol = choice([
        just("==").to(Token::DoubleEqual),
        just("!=").to(Token::Neq),
        just("<=").to(Token::Le),
        just(">=").to(Token::Ge),
        just("<<").to(Token::DoubleLt),
        just(">>").to(Token::DoubleGt),
        just("&&").to(Token::DoubleAmpersand),
        just("||").to(Token::DoublePipe),
        just(":").to(Token::Colon),
        just(",").to(Token::Comma),
        just("(").to(Token::LParen),
        just(")").to(Token::RParen),
        just("{").to(Token::LBrace),
        just("}").to(Token::RBrace),
        just("<").to(Token::Lt),
        just(">").to(Token::Gt),
        just("=").to(Token::Equal),
        just("+").to(Token::Plus),
        just("-").to(Token::Minus),
        just("*").to(Token::Asterisk),
        just("/").to(Token::Slash),
        just("%").to(Token::Percent),
        just("!").to(Token::Exclamation),
        just("&").to(Token::Ampersand),
        just("|").to(Token::Pipe),
        just("^").to(Token::Caret),
        just("~").to(Token::Tilde),
        just(".").to(Token::Dot),
    ])
    .labelled("symbol");

    let newline = just('\n').to(Token::Eol);
    let whitespace = any()
        .filter(|c: &char| c.is_whitespace() && *c != '\n')
        .ignored();

    let comment = just(';')
        .then(any().and_is(just('\n').not()).repeated())
        .ignored()
        .labelled("comment");

    let comments_and_spaces = comment.or(whitespace).repeated();

    let token = choice((newline, keyword, identifier, number, symbol));

    let lexer = token
        .map_with(|t, e| (t, e.span()))
        .padded_by(comments_and_spaces)
        // .recover_with(skip_then_retry_until(any().ignored(), end()))
        .repeated()
        .collect()
        .then_ignore(comments_and_spaces);
    lexer
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;
    use test_strategy::proptest;

    pub fn tokenize<'src>(
        input: &'src str,
        errors: &mut Vec<AssemblerError>,
    ) -> Option<Vec<Token<'src>>> {
        super::tokenize(input, None, errors).map(|v| v.into_iter().map(|(t, _)| t).collect())
    }

    #[test_case("abcd", &[Token::Identifier("abcd")]; "identifier_simple")]
    #[test_case("abcd1", &[Token::Identifier("abcd1")]; "identifier_with_number")]
    #[test_case("_123", &[Token::Identifier("_123")]; "identifier_numerical")]
    #[test_case("foo!", &[Token::MacroCall("foo")]; "macro_call")]
    #[test_case("const", &[Token::Const]; "keyword_const")]
    #[test_case("macro", &[Token::Macro]; "keyword_macro")]
    #[test_case("123", &[Token::Number(123)]; "number_decimal")]
    #[test_case("9223372036854775807", &[Token::Number(9223372036854775807)]; "number_max")]
    #[test_case("0x2a", &[Token::Number(0x2a)]; "number_hex")]
    #[test_case("0b1010", &[Token::Number(0b1010)]; "number_binary")]
    #[test_case("==", &[Token::DoubleEqual]; "symbol_eq")]
    #[test_case("<=", &[Token::Le]; "symbol_le")]
    #[test_case("{ }", &[Token::LBrace, Token::RBrace]; "braces")]
    #[test_case("a\nb", &[Token::Identifier("a"), Token::Eol, Token::Identifier("b")]; "newline")]
    #[test_case("; c", &[]; "only_comment1")]
    #[test_case("; c\n", &[Token::Eol]; "only_comment2")]
    #[test_case("a ; comment\nb", &[Token::Identifier("a"), Token::Eol,  Token::Identifier("b")]; "line_comment")]
    #[test_case("a ; comment", &[Token::Identifier("a")]; "line_comment_at_end")]
    #[test_case("", &[]; "empty_input")]
    #[test_case("123 + 456", &[Token::Number(123), Token::Plus, Token::Number(456)]; "addition_example")]
    fn tokenize_examples(input: &str, expected: &[Token]) {
        let mut errors = Vec::new();
        let tokens = tokenize(input, &mut errors);
        assert!(errors.is_empty());
        assert_eq!(tokens.unwrap(), expected);
    }

    #[test_case("0x"; "num_only_prefix")]
    #[test_case("9223372036854775808"; "number_max_plus_one")]
    #[test_case(r"0xefg123"; "num_hex_wrong_character")]
    #[test_case(r"0b2"; "num_bin_wrong_character")]
    fn tokenize_errors(input: &str) {
        let mut errors = Vec::new();
        let tokens = tokenize(input, &mut errors);
        dbg!(&tokens);
        assert!(!errors.is_empty());
    }

    #[proptest]
    fn comment(#[strategy(r"abc ;[^\n]*\ndef")] input: String) {
        let mut errors = Vec::new();
        let tokens: Vec<_> = tokenize(input.as_ref(), &mut errors).unwrap();
        assert!(errors.is_empty());
        assert_eq!(
            tokens,
            &[
                Token::Identifier("abc"),
                Token::Eol,
                Token::Identifier("def")
            ]
        );
    }
}
