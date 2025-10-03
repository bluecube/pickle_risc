mod assembler;
mod chumsky_util;
mod lexer;
mod parser;
mod types;

use ariadne::{Color, Label, Report, ReportKind};
use clap::Parser as _;

use std::{collections::HashMap, fs, io, path::PathBuf};

use crate::{
    assembler::Assembler,
    types::{AssemblerError, FileId, Span},
};

// use assembler::{AsmResult, AssemblerState, files::InputFiles};

#[derive(clap::Parser, Debug)]
struct Cli {
    /// Paths to input assembler files
    input_files: Vec<PathBuf>,

    /// Path to the output file
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Debug)]
struct AriadneCache<'assembler> {
    assembler: &'assembler Assembler,
    cache: HashMap<FileId, ariadne::Source<String>>,
    empty_source: ariadne::Source<String>,
}

impl<'assembler> ariadne::Cache<Option<FileId>> for AriadneCache<'assembler> {
    type Storage = String;

    fn fetch(
        &mut self,
        id: &Option<FileId>,
    ) -> Result<&ariadne::Source<Self::Storage>, impl std::fmt::Debug> {
        Ok::<_, io::Error>(if let Some(id) = id {
            match self.cache.entry(*id) {
                std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                    occupied_entry.into_mut()
                }
                std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                    let path = self
                        .assembler
                        .get_path(*id)
                        .expect("It should be impossible to get an invalid FileId");
                    let content = fs::read_to_string(path)?;
                    vacant_entry.insert(content.into())
                }
            }
        } else {
            &self.empty_source
        })
    }

    fn display<'a>(&self, id: &'a Option<FileId>) -> Option<impl std::fmt::Display + 'a> {
        if let Some(id) = id {
            let path = self
                .assembler
                .get_path(*id)
                .expect("It should be impossible to get an invalid FileId");
            Some(format!("{}", path.display()))
        } else {
            Some("<no file>".to_string())
        }
    }
}

impl<'assembler> AriadneCache<'assembler> {
    fn new(assembler: &Assembler) -> AriadneCache<'_> {
        AriadneCache {
            assembler,
            cache: Default::default(),
            empty_source: String::new().into(),
        }
    }
}

impl ariadne::Span for &Span {
    type SourceId = Option<FileId>;

    fn source(&self) -> &Self::SourceId {
        &self.file_id
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}

fn print_error(error: &'_ AssemblerError) -> ariadne::Report<'_, &'_ Span> {
    match error {
        AssemblerError::InvalidToken(err) | AssemblerError::SyntaxError(err) => {
            let mut report = Report::build(ReportKind::Error, &err.span)
                .with_message(format!(
                    "syntax error: found {}",
                    err.found.as_deref().unwrap_or("end of input")
                ))
                .with_label(
                    Label::new(&err.span)
                        .with_message("error occurred here")
                        .with_color(Color::Red),
                );

            for (ctx, span) in err.context.iter() {
                report = report.with_label(
                    Label::new(span)
                        .with_message(ctx.clone())
                        .with_color(Color::Yellow),
                );
            }

            report
        }

        AssemblerError::NestedMacro {
            span,
            nested_in_name,
            nested_in_span,
        } => Report::build(ReportKind::Error, span)
            .with_message(format!("nested macro inside `{}`", nested_in_name))
            .with_label(
                Label::new(span)
                    .with_message("macro defined here")
                    .with_color(Color::Red),
            )
            .with_label(
                Label::new(nested_in_span)
                    .with_message("enclosing macro")
                    .with_color(Color::Yellow),
            ),

        AssemblerError::FileOpenFailed {
            span,
            file_path,
            error,
        } => Report::build(
            ReportKind::Error,
            span.as_ref().unwrap_or_else(|| &Span {
                file_id: None,
                start: 0,
                end: 0,
            }),
        )
        .with_message(format!(
            "could not open file {}: {}",
            file_path.display(),
            error
        )),
    }
    .finish()
}

fn main() {
    let cli = Cli::parse();

    let mut assembler = Assembler::default();
    let mut errors = Vec::new();

    for file_name in cli.input_files {
        let _ = assembler.add_file(file_name, None, &mut errors);
    }

    if errors.is_empty() {
        dbg!(assembler.assemble());
    }

    let mut sources = AriadneCache::new(&assembler);

    for err in errors {
        print_error(&err).eprint(&mut sources).unwrap();
    }
}
