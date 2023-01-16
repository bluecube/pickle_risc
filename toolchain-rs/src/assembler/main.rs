use clap::Parser;

use std::path::PathBuf;

use pickle_toolchain::assembler::AssemblerState;

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

    let mut state = AssemblerState::new();

    //let mut file_sources = SimpleFiles::new();

    //for path in cli.input_files {
    //    state.assemble_file(path)
    //}

    state.start_second_pass();

    Ok(())
}
