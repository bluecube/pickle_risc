use clap::Parser;

use codespan_reporting::{
    diagnostic::Diagnostic,
    term::{
        emit,
        termcolor::{ColorChoice, StandardStream},
    },
};
use std::path::PathBuf;

use pickle_toolchain::assembler::{files::InputFiles, AsmResult, AssemblerState};

#[derive(Parser, Debug)]
struct Cli {
    /// Paths to input assembler files
    input_files: Vec<PathBuf>,

    /// Path to the output file
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn assemble(paths: Vec<impl Into<PathBuf>>, files: &mut InputFiles) -> AsmResult<()> {
    let mut state = AssemblerState::new();

    for path in paths {
        files.add_file(path)?;
    }

    state.assemble(files)
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut files = InputFiles::new();
    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();

    if let Err(e) = assemble(cli.input_files, &mut files) {
        let diagnostic: Diagnostic<_> = e.into();
        emit(&mut writer.lock(), &config, &files, &diagnostic)?;
    }

    Ok(())
}
