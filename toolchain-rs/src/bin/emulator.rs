use clap::Parser;
use pickle_toolchain::cpu::CpuState;
use pickle_toolchain::cpu_types::*;
use pickle_toolchain::instruction::Instruction;
use pickle_toolchain::system::SystemState;

use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Cli {
    /// Path to intel hex image of the boot rom
    boot_rom_path: PathBuf,

    /// Initialize the system with random state
    #[arg(short, long)]
    randomize: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut system = if cli.randomize {
        let mut rng = rand::thread_rng();
        SystemState::with_rng(cli.boot_rom_path, &mut rng)
    } else {
        SystemState::new(cli.boot_rom_path)
    }?;

    loop {
        print_cpu_state(&system.cpu);
        match system.step() {
            Ok(()) => (),
            Err(EmulatorError::Break) => {
                println!("Break");
                return Ok(());
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }
}

fn print_cpu_state(state: &CpuState) {
    for i in 0..8 {
        let v = state.get_gpr(Gpr::new(i));
        print!("r{}: {}({:#06x}) ", i, v, v);
    }
    println!();

    let instruction = state.get_next_instruction();
    print!(
        "{:#06x}/{}: {:04x}",
        state.get_pc(),
        state.get_step(),
        instruction,
    );
    if let Ok(instruction) = Instruction::try_from(instruction) {
        print!(" {}", instruction);
    }
    println!();
}
