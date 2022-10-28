mod cpu;
mod cpu_types;
mod memory;
mod system;
mod util;

use ux::*; // Non-standard integer types

use crate::system::SystemState;
use crate::cpu::CpuState;
use clap::Parser;

use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Cli {
    /// Path to intel hex image of the boot rom
    boot_rom_path: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut system = SystemState::new(cli.boot_rom_path)?;

    loop {
        print_cpu_state(&system.cpu);
        system.step()?;
    }

    Ok(())
}

fn print_cpu_state(state: &CpuState) {
    for i in 0..8 {
        let v = state.get_gpr(u3::new(i));
        print!("r{}: {}({:06x})", i, v, v);
    }
    println!();
    println!("pc: {}", state.get_pc());
}
