use std::{io, path::PathBuf};

use crate::assembler::QualifiedName;

pub use crate::assembler::FileId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub file_id: Option<FileId>,
    pub start: usize,
    pub end: usize,
}

pub type Spanned<T> = (T, Span);

#[derive(Debug)]
pub struct ParseError {
    pub span: Span,
    pub found: Option<String>,
    pub expected: Vec<String>,
    pub context: Vec<(String, Span)>,
}

#[derive(Debug)]
pub enum AssemblerError {
    InvalidToken(ParseError),
    SyntaxError(ParseError),
    NestedMacro {
        span: Span,
        nested_in_name: QualifiedName,
        nested_in_span: Span,
    },
    FileOpenFailed {
        span: Option<Span>,
        file_path: PathBuf,
        error: io::Error,
    },
}
