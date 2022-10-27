mod cpu;
mod cpu_types;
mod memory;
mod system;
mod util;

use crate::system::SystemState;
use clap::Parser;

use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Cli {
    /// Path to intel hex image of the boot rom
    boot_rom_path: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let _system = SystemState::new(cli.boot_rom_path)?;

    Ok(())
}
