use clap::Parser;
use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    files::SimpleFiles,
    term::termcolor::{ColorChoice, StandardStream},
};

use std::path::PathBuf;

use pickle_toolchain::assembler::lexer::tokenize_str;

#[derive(Parser, Debug)]
struct Cli {
    /// Paths to input assembler files
    input_files: Vec<PathBuf>,

    /// Path to the output file
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    //let mut file_sources = SimpleFiles::new();

    for path in cli.input_files {
        let file_str = std::fs::read_to_string(&path)?;
        for (tok, span) in tokenize_str(&file_str) {
            println!("{:?} ({:?})", tok, span);
        }
    }

    Ok(())
}
