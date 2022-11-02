use clap::Parser;
use pickle_toolchain::image::load_ihex;
use pickle_toolchain::instruction::Instruction;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Cli {
    /// Path to intel hex image of the boot rom
    image_path: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let img = load_ihex(cli.image_path)?;
    for (address, v) in img.iter().enumerate() {
        print!("{:#06x}: ", address);
        match Instruction::try_from(*v) {
            Ok(instruction) => println!("{}", instruction),
            Err(_) => println!("<bad instruction>"),
        }
    }
    Ok(())
}
