use std::fmt::Debug;

use chumsky::error::Rich;

use crate::types::{FileId, ParseError, Span};

impl chumsky::span::Span for Span {
    type Context = Option<FileId>;
    type Offset = usize;

    fn new(context: Self::Context, range: std::ops::Range<Self::Offset>) -> Self {
        Span {
            file_id: context,
            start: range.start,
            end: range.end,
        }
    }

    fn context(&self) -> Self::Context {
        self.file_id
    }

    fn start(&self) -> Self::Offset {
        self.start
    }

    fn end(&self) -> Self::Offset {
        self.end
    }
}

impl<'a, T: Debug> From<Rich<'a, T, Span>> for ParseError {
    fn from(value: Rich<'a, T, Span>) -> Self {
        ParseError {
            span: value.span().clone(),
            found: value.found().map(|found| format!("{:?}", found)),
            expected: value
                .expected()
                .into_iter()
                .map(|pattern| format!("{:?}", pattern))
                .collect(),
            context: value
                .contexts()
                .into_iter()
                .map(|(pattern, span)| (format!("{:?}", pattern), span.clone()))
                .collect(),
        }
    }
}
