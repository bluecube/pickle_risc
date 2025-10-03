use std::{
    collections::HashMap,
    fmt::Display,
    fs,
    path::{Path, PathBuf},
};

use id_arena::{Arena, Id};
use toolchain_core::instruction::Instruction;

use crate::{
    lexer,
    parser::{self, Ast, Item},
    types::{AssemblerError, Span, Spanned},
};

pub type FileId = Id<ParsedFile>;

#[derive(Clone, Debug, Default)]
pub struct Assembler {
    files: Arena<ParsedFile>,
}

#[derive(Clone, Debug)]
pub struct ParsedFile {
    path: PathBuf,

    /// Ast might be empty if parsing failed, but we still need the ParsedFile and FileId to report errors.
    ast: Option<Ast>,
}

fn parse_file(
    path: &Path,
    file_id: FileId,
    source_span: Option<Span>,
    errors: &mut Vec<AssemblerError>,
) -> Option<Ast> {
    let content = fs::read_to_string(&path)
        .map_err(|error| {
            errors.push(AssemblerError::FileOpenFailed {
                span: source_span,
                file_path: path.to_owned(),
                error,
            })
        })
        .ok()?;
    let tokens = lexer::tokenize(content.as_str(), Some(file_id), errors)?;
    parser::parse(tokens.as_slice(), Some(file_id), content.len(), errors)
}

impl Assembler {
    /// Add a a file to to be assembled.
    /// Returns file ID that is used in the error reports.
    pub fn add_file(
        &mut self,
        path: PathBuf,
        source_span: Option<Span>,
        errors: &mut Vec<AssemblerError>,
    ) -> FileId {
        let file_id = self.files.alloc_with_id(|file_id| {
            let ast = parse_file(&path, file_id, source_span, errors);
            ParsedFile { path, ast }
        });

        file_id
    }

    pub fn assemble(&self) -> Vec<Instruction> {
        let macros = self.collect_macros();
        dbg!(macros);
        Vec::new()
    }

    pub fn get_path(&self, file_id: FileId) -> Option<&Path> {
        self.files.get(file_id).map(|file| file.path.as_ref())
    }

    fn collect_macros(&'_ self) -> AssemblerTable<MacroDef<'_>> {
        let mut table = AssemblerTable::default();

        for (i, (_file_id, f)) in self.files.iter().enumerate() {
            let Some(ast) = f.ast.as_ref() else { continue };
            let mut current_scope = QualifiedName::new_anonymous(i);
            collect_macros_recursive(ast, &mut current_scope, &mut table);
        }

        table
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum QualifiedNameEntry {
    /// Anonymous entry, parameter is just for disambiguation
    Anonymous(usize),
    Named(String),
}

impl<'src> Display for QualifiedNameEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualifiedNameEntry::Anonymous(id) => write!(f, "<anonymous {}>", id),
            QualifiedNameEntry::Named(name) => write!(f, "{}", name),
        }
    }
}

#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct QualifiedName(Vec<QualifiedNameEntry>);

impl QualifiedName {
    fn new_anonymous(id: usize) -> Self {
        QualifiedName(vec![QualifiedNameEntry::Anonymous(id)])
    }
    fn push_name(&mut self, name: String) {
        self.0.push(QualifiedNameEntry::Named(name));
    }

    fn push_anonymous(&mut self, id: usize) {
        self.0.push(QualifiedNameEntry::Anonymous(id));
    }

    fn pop(&mut self) {
        self.0.pop();
    }
}

impl<'src> Display for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut it = self.0.iter();

        let Some(first) = it.next() else {
            return Ok(());
        };
        write!(f, "{}", first)?;
        for entry in it {
            write!(f, ".{}", entry)?;
        }

        Ok(())
    }
}

impl<'src> std::fmt::Debug for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "QualifiedName({})", self)
    }
}

#[derive(Clone, Debug)]
pub struct AssemblerTable<T>(HashMap<QualifiedName, T>);

impl<'src, T> Default for AssemblerTable<T> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

#[derive(Clone, Debug)]
pub struct MacroDef<'ast> {
    span: Span,
    params: &'ast Vec<String>,
    body: &'ast Vec<Spanned<Item>>,
}

fn collect_macros_recursive<'ast>(
    ast: &'ast Ast,
    current_scope: &mut QualifiedName,
    table: &mut AssemblerTable<MacroDef<'ast>>,
) {
    for (index, (item, span)) in ast.iter().enumerate() {
        match item {
            Item::Scope { label, content } => {
                if let Some(label) = label {
                    current_scope.push_name(label.clone());
                } else {
                    current_scope.push_anonymous(index);
                }
                collect_macros_recursive(content, current_scope, table);
                current_scope.pop();
            }
            Item::MacroDefinition { name, params, body } => {
                let mut macro_name = current_scope.clone();
                macro_name.push_name(name.clone());
                table.0.insert(
                    macro_name,
                    MacroDef {
                        span: span.clone(),
                        params,
                        body,
                    },
                );
            }
            _ => (),
        }
    }
}
