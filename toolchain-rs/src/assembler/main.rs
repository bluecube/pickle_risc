use clap::Parser;

use std::path::PathBuf;

use pickle_toolchain::assembler::lexer::{Logos, Token};

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

    for path in cli.input_files {
        let file_str = std::fs::read_to_string(&path)?;
        let lexer = Token::lexer(&file_str);
        for (tok, span) in lexer.spanned() {
            println!("{:?} ({:?})", tok, span);
        }
    }

    Ok(())
}
