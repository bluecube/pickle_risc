use codespan_reporting::{
    diagnostic::{Label, LabelStyle},
    files::{line_starts, Error, Files},
};
use id_arena::Arena;
use logos::Logos;
use more_asserts::*;
use std::fs::read_to_string;
use std::ops::Range;
use std::path::PathBuf;

use crate::assembler::{
    lexer::{Span, Token},
    AsmError, AsmResult,
};

#[derive(Default)]
pub struct InputFiles {
    files: Arena<File>,
}

pub type FileId = id_arena::Id<File>;

impl<'a> Files<'a> for InputFiles {
    type FileId = id_arena::Id<File>;
    type Name = &'a str;
    type Source = &'a str;

    fn name(&'a self, id: Self::FileId) -> Result<Self::Name, Error> {
        Ok(&self.files[id].name)
    }

    fn source(&'a self, id: Self::FileId) -> Result<Self::Source, Error> {
        Ok(&self.files[id].source)
    }

    fn line_index(&'a self, id: Self::FileId, byte_index: usize) -> Result<usize, Error> {
        Ok(self.files[id]
            .line_starts
            .binary_search(&byte_index)
            .unwrap_or_else(|next_line| next_line - 1))
    }

    fn line_range(&'a self, id: Self::FileId, line_index: usize) -> Result<Range<usize>, Error> {
        let file = &self.files[id];
        let Some(start) = file.line_starts.get(line_index) else {
            return Err(Error::LineTooLarge { given: line_index, max: file.line_starts.len() - 1 });
        };
        let end = match file.line_starts.get(line_index + 1) {
            Some(end) => *end,
            None => file.source.len(),
        };

        Ok(*start..end)
    }
}

pub struct File {
    name: String,
    tokens: Vec<(Token, Span)>,
    source: String,
    line_starts: Vec<usize>,
}

impl InputFiles {
    pub fn new() -> InputFiles {
        return Default::default();
    }

    pub fn add_file(&mut self, path: impl Into<PathBuf>) -> AsmResult<FileId> {
        let path = path.into();
        let source = read_to_string(&path).map_err(|e| AsmError::IoError {
            description: format!("{e}"),
        })?;
        self.add_snippet(format!("{}", path.display()), source)
    }

    pub fn add_snippet(&mut self, name: String, source: String) -> AsmResult<FileId> {
        let tokens = Token::lexer(&source).spanned().collect();
        let line_starts = line_starts(&source).collect();
        Ok(self.files.alloc(File {
            name,
            tokens,
            source,
            line_starts,
        }))
    }

    pub fn tokens<'a>(&'a self, file_id: FileId) -> FileTokens<'a> {
        FileTokens {
            file_id,
            tokens: self.files[file_id].tokens.as_slice(),
        }
    }

    pub fn iter_file_tokens<'a>(&'a self) -> impl Iterator<Item = FileTokens<'a>> {
        self.files.iter().map(|(file_id, file)| FileTokens {
            file_id,
            tokens: file.tokens.as_slice(),
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub start: usize,
    pub end: usize,
    pub file_id: FileId,
}

impl Location {
    /// Combine the two locations into one that covers from start of the first one
    /// to the end of the second one.
    /// Assumes that both locations are in the same file and that they are ordered (
    /// start of first is earlier than end of second).
    /// Either of the two locations may be empty.
    pub fn extend_to(&self, other: &Location) -> Location {
        assert_eq!(self.file_id, other.file_id);

        if self.start == self.end {
            return *other;
        } else if other.start == other.end {
            return *self;
        }

        assert_le!(self.start, other.end);

        Location {
            start: self.start,
            end: other.end,
            file_id: self.file_id,
        }
    }

    pub fn to_label(&self, style: LabelStyle) -> Label<FileId> {
        Label::new(style, self.file_id, self.start..self.end)
    }

    pub fn to_primary_label(&self) -> Label<FileId> {
        self.to_label(LabelStyle::Primary)
    }

    pub fn to_secondary_label(&self) -> Label<FileId> {
        self.to_label(LabelStyle::Secondary)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FileTokens<'a> {
    tokens: &'a [(Token, Span)],
    file_id: FileId,
}

impl<'a> FileTokens<'a> {
    pub fn first(&self) -> Option<(&'a Token, Location)> {
        let (token, span) = self.tokens.first()?;
        Some((
            token,
            Location {
                start: span.start,
                end: span.end,
                file_id: self.file_id,
            },
        ))
    }

    pub fn rest(&self) -> FileTokens<'a> {
        FileTokens {
            tokens: &self.tokens[1..],
            file_id: self.file_id,
        }
    }

    pub fn empty_location(&self) -> Location {
        Location {
            start: 0,
            end: 0,
            file_id: self.file_id,
        }
    }
}

#[cfg(test)]
/// Helper for unit tests
pub struct SnippetTokenizer {
    files: InputFiles,
    snippet_id: FileId,
}

#[cfg(test)]
impl SnippetTokenizer {
    pub fn new(source: String) -> SnippetTokenizer {
        let mut files = InputFiles::new();
        let snippet_id = files.add_snippet("<input>".to_owned(), source).unwrap();
        SnippetTokenizer { files, snippet_id }
    }

    pub fn tokens<'a>(&'a self) -> FileTokens<'a> {
        self.files.tokens(self.snippet_id)
    }

    pub fn location(&self, start: usize, end: usize) -> Location {
        Location {
            start,
            end,
            file_id: self.snippet_id,
        }
    }
}
