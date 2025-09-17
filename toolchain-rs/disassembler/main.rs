use clap::Parser;
use std::path::PathBuf;
use toolchain_core::image::load_ihex;

use crate::disassembler::Disassembler;

mod disassembler;

#[derive(Parser, Debug)]
struct Cli {
    /// Path to intel hex image of the boot rom
    image_path: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let img = load_ihex(cli.image_path)?;
    for entry in Disassembler::new(&img) {
        println!("{}", entry);
    }
    Ok(())
}
