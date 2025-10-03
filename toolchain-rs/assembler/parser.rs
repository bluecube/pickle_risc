use chumsky::{
    input::{BorrowInput, Input},
    prelude::*,
};

use crate::{
    lexer::Token,
    types::{AssemblerError, FileId, Span, Spanned},
};

pub type Ast = Vec<Spanned<Item>>;

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Scope {
        label: Option<String>,
        content: Ast,
    },
    Instruction {
        name: String,
        args: Vec<Spanned<Expr>>,
    },
    MacroCall {
        name: String,
        args: Vec<Spanned<Expr>>,
    },
    MacroDefinition {
        name: String,
        params: Vec<String>,
        body: Ast,
    },
    Label {
        name: String,
    },
    Const {
        name: String,
        value: Spanned<Expr>,
    },
}

/// Expressions for instruction arguments and constants
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(i64),
    QualifiedName(Vec<Spanned<String>>),
    BinaryOp {
        op: BinOp,
        lhs: Box<Spanned<Expr>>,
        rhs: Box<Spanned<Expr>>,
    },
    UnaryOp {
        op: UnOp,
        expr: Box<Spanned<Expr>>,
    },
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Shl,
    Shr,
    BitAnd,
    BitOr,
    BitXor,
    And,
    Or,
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
    BitNot,
}

pub fn parse<'src>(
    tokens: &'src [Spanned<Token<'src>>],
    file_id: Option<FileId>,
    input_len: usize,
    errors: &mut Vec<AssemblerError>,
) -> Option<Ast> {
    let input = tokens.map(
        Span {
            file_id,
            start: input_len,
            end: input_len,
        },
        |(token, span)| (token, span),
    );
    let (ast, parse_errors) = parser().parse(input).into_output_errors();
    errors.extend(
        parse_errors
            .into_iter()
            .map(|e| AssemblerError::SyntaxError(e.into())),
    );
    ast
}

fn parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Ast, extra::Err<Rich<'tokens, Token<'src>, Span>>>
where
    I: BorrowInput<'tokens, Token = Token<'src>, Span = Span>,
{
    recursive(|ast| {
        let identifier = select! { Token::Identifier(name) => name }.map(ToOwned::to_owned);
        let scoped_ast = ast.delimited_by(just(Token::LBrace), just(Token::RBrace));

        let label_name = identifier.then_ignore(just(Token::Colon));
        let scope = label_name
            .clone()
            .or_not()
            .then(scoped_ast.clone())
            .map(|(label, items)| Item::Scope {
                label: label.map(|x| x.to_owned()),
                content: items,
            })
            .labelled("scope");

        let expression = expression_parser();

        let instruction_tail = expression
            .clone()
            .separated_by(just(Token::Comma))
            .collect()
            .then_ignore(choice((
                just(Token::Eol).ignored(),
                just(Token::RBrace).rewind().ignored(),
                end().ignored(),
            )));
        let instruction = identifier
            .then(instruction_tail.clone())
            .map(|(name, args)| Item::Instruction { name, args })
            .labelled("instruction")
            .as_context();
        let macro_call = select! { Token::MacroCall(name) => name }
            .then(instruction_tail)
            .map(|(name, args)| Item::MacroCall {
                name: name.to_owned(),
                args,
            })
            .labelled("macro call")
            .as_context();

        let macro_def = just(Token::Macro)
            .ignore_then(group((
                identifier,
                identifier.separated_by(just(Token::Comma)).collect(),
                scoped_ast,
            )))
            .map(|(name, params, body)| Item::MacroDefinition { name, params, body })
            .labelled("macro definition")
            .as_context();

        let label = label_name
            .map(|name| Item::Label { name })
            .labelled("label definition");

        let constant = just(Token::Const)
            .ignore_then(identifier)
            .then_ignore(just(Token::DoubleEqual))
            .then(expression)
            .map(|(name, value)| Item::Const { name, value })
            .labelled("constant definition");

        just(Token::Eol)
            .repeated()
            .ignore_then(choice((
                scope,
                instruction,
                macro_call,
                macro_def,
                label,
                constant,
            )))
            .map_with(|item, e| (item, e.span()))
            .repeated()
            .collect()
            .then_ignore(just(Token::Eol).repeated())
    })
}

fn expression_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Spanned<Expr>, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
    I: BorrowInput<'tokens, Token = Token<'src>, Span = Span>,
{
    recursive(|expression| {
        let atom = choice((
            select! { Token::Number(i) => i }.map_with(|i, e| (Expr::Number(i), e.span())),
            select! { Token::Identifier(name) => name }
                .map_with(|name, e| (name.to_owned(), e.span()))
                .separated_by(just(Token::Dot))
                .at_least(1)
                .collect()
                .map_with(|names, e| (Expr::QualifiedName(names), e.span()))
                .labelled("qualified name")
                .as_context(),
            expression.delimited_by(just(Token::LParen), just(Token::RParen)),
        ));

        use chumsky::pratt;
        let prefix = |precedence, token, un_op| {
            pratt::prefix(precedence, just(token), move |_op, expr, extra| {
                (
                    Expr::UnaryOp {
                        op: un_op,
                        expr: Box::new(expr),
                    },
                    extra.span(),
                )
            })
        };
        let infix = |precedence, token, bin_op| {
            pratt::infix(
                pratt::left(precedence),
                just(token),
                move |x, _op, y, extra| {
                    (
                        Expr::BinaryOp {
                            op: bin_op,
                            lhs: Box::new(x),
                            rhs: Box::new(y),
                        },
                        extra.span(),
                    )
                },
            )
        };
        atom.pratt((
            prefix(9, Token::Minus, UnOp::Neg),
            prefix(9, Token::Exclamation, UnOp::Not),
            prefix(9, Token::Tilde, UnOp::BitNot),
            infix(8, Token::Asterisk, BinOp::Mul),
            infix(8, Token::Slash, BinOp::Div),
            infix(8, Token::Percent, BinOp::Mod),
            infix(7, Token::Plus, BinOp::Add),
            infix(7, Token::Minus, BinOp::Sub),
            infix(6, Token::DoubleLt, BinOp::Shl),
            infix(6, Token::DoubleGt, BinOp::Shr),
            infix(5, Token::Ampersand, BinOp::BitAnd),
            infix(4, Token::Caret, BinOp::BitXor),
            infix(3, Token::Pipe, BinOp::BitOr),
            infix(2, Token::DoubleEqual, BinOp::Eq),
            infix(2, Token::Neq, BinOp::Neq),
            infix(2, Token::Lt, BinOp::Lt),
            infix(2, Token::Gt, BinOp::Gt),
            infix(2, Token::Le, BinOp::Le),
            infix(2, Token::Ge, BinOp::Ge),
            infix(1, Token::DoubleAmpersand, BinOp::And),
            infix(0, Token::DoublePipe, BinOp::Or),
        ))
    })
    .labelled("expression")
    .as_context()
}
