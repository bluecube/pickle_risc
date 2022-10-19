use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::iter::repeat;
use std::ops::Range;
use std::collections::HashMap;

use either;
use thiserror::Error;
use itertools::Itertools;
use anyhow;
use instruction_set::{InstructionSet, Instruction};

fn main() {
    generate_instruction_handler().unwrap();
}

fn generate_instruction_handler() -> anyhow::Result<()> {
    let instruction_bits = 16;
    let opcode_bits = 7;

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let target_path = Path::new(&out_dir).join("instruction_handler.rs");
    let definition_path = Path::new("..").join("instruction_set.json5");

    println!("cargo:warning=Output goes to {}", target_path.to_str().unwrap());
    println!("cargo:rerun-if-changed={}", definition_path.to_str().unwrap());

    let definition = InstructionSet::load(definition_path)?;
    let mut target = File::create(target_path)?;

    writeln!(target, "#[allow(unreachable_code)]")?;
    writeln!(target, "match opcode >> {} {{", instruction_bits - opcode_bits)?;
    let opcode_table = make_opcode_table(&definition, opcode_bits, instruction_bits)?;
    for (count, (first_opcode, instruction)) in opcode_table
        .iter()
        .enumerate()
        .dedup_by_with_count(|x, y| x.1.map(|x| x.0) == y.1.map(|x| x.0))
    {

        generate_opcode_match_arm(
            instruction.map(|x| x.0),
            first_opcode..(first_opcode + count),
            if let Some((_, instruction_def)) = instruction {
                &instruction_def.microcode
                // Unwrap is ok, because we got the menmonic from the encodings in the first place
            } else {
                &definition.invalid_instruction_microcode
            },
            &definition.substitutions,
            &mut target
        )?;
    }
    writeln!(target, "    _ => unreachable!(),")?;
    writeln!(target, "}}")?;

    Ok(())
}

fn make_opcode_table(definition: &InstructionSet, opcode_bits: usize, instruction_bits: usize) -> anyhow::Result<Vec<Option<(&str, &Instruction)>>> {
    let mut table: Vec<Option<(&str, &Instruction)>> = repeat(None).take(1 << opcode_bits).collect();

    for (mnemonic, instruction_def) in &definition.instructions {
        let encoding = instruction_def.encoding(&mnemonic, instruction_bits)?;
        for opcode in expand_encoding(&encoding[..opcode_bits]) {
            table[opcode] = Some((&mnemonic, &instruction_def));
        }
    }

    Ok(table)
}

/// Converts a str with 0, 1 and other into all numbers that match this bit string
fn expand_encoding(s: &str) -> impl Iterator<Item = usize> {
    s.chars()
        .map(|c| match c { '0' => 0..=0, '1' => 1..=1, _ => 0..=1, })
        .multi_cartesian_product()
        .map(|x| x.iter().fold(0, |acc, digit| (acc << 1) + digit))
}

fn generate_opcode_match_arm(
    mnemonic: Option<&str>,
    opcodes: Range<usize>,
    microcode: &Option<Vec<Vec<String>>>,
    substitutions: &HashMap<String, Vec<String>>,
    target: &mut File
) -> anyhow::Result<()> {
    if opcodes.len() == 1 {
        writeln!(target, "    {:#04x} => {{", opcodes.start)?;
    } else {
        writeln!(target, "    {:#04x}..={:#04x} => {{", opcodes.start, opcodes.end - 1)?;
    }
    writeln!(target, "        // {}", mnemonic.unwrap_or("invalid instruction"))?;
    if let Some(microcode) = microcode {
        for (i, microcode_step) in microcode.iter().enumerate() {
            generate_microcode_step(i, microcode_step, substitutions, target)?;
        }
    } else {
        writeln!(target, "        todo!(); // Missing microcode!")?;
    }
    writeln!(target, "    }}")?;

    Ok(())
}

fn generate_microcode_step(
    step: usize,
    microcode: &Vec<String>,
    substitutions: &HashMap<String, Vec<String>>,
    target: &mut File
) -> anyhow::Result<()> {
    const INDENT: &str = "        ";

    writeln!(target, "{}{{ // Microcode step {}", INDENT, step)?;
    writeln!(target, "{}    #[allow(unused_mut,unused_variables)] let mut segment = VirtualMemorySegment::Data;", INDENT)?;

    let mut microinstructions: Vec<(String, usize)> = Vec::new();
    for microinstruction in microcode {
        for expanded in substitute_microinstruction(microinstruction, substitutions)? {
            microinstructions.push(translate_microinstruction(expanded)?);
        }
    }
    microinstructions.sort_by(|(_, phase1), (_, phase2)| phase1.cmp(phase2));

    for (code, _) in microinstructions {
        writeln!(target, "{}    {}", INDENT, code)?;
    }

    writeln!(target, "{}}}", INDENT)?;

    // TODO: Conditionals in microcode
    Ok(())
}

fn substitute_microinstruction<'a>(microinstruction: &'a str, substitutions: &'a HashMap<String, Vec<String>>) -> Result<impl Iterator<Item = &'a str>, CodegenError> {
    if microinstruction.starts_with("$") {
        if let Some(subst) = substitutions.get(&microinstruction[1..]) {
            Ok(either::Left(subst.into_iter().map(|x| x.as_str())))
        } else {
            Err(CodegenError::BadSubstitution(microinstruction.to_owned()))
        }
    } else {
        Ok(either::Right(std::iter::once(microinstruction)))
    }
}

/// Parse a microinstruction, returns rust code to emulate it, and its phase
/// (to produce microinstructions that produce value before the ones that consume them)
fn translate_microinstruction(microinstruction: &str) -> Result<(String, usize), CodegenError> {
    let (code, priority) = match microinstruction {
        "pc->left" => ("let left_bus = self.pc;", 0),
        "pc->addr_base" => ("let addr_base_bus = self.pc;", 0),
        "zero->left" => ("let left_bus = 0;", 0),
        "f2->left" => ("let left_bus = self.get_gpr(field!(opcode, Gpr));", 0),
        "f3->left" => ("let left_bus = self.get_gpr(field!(opcode >> 3, Gpr));", 0),
        "f4->right" => ("let right_bus = self.get_gpr(field!(opcode >> 6, Gpr));", 0),
        "f5->right" => ("let right_bus = self.get_gpr(field!(opcode >> 10, Gpr));", 0),
        "f6->right" => ("let right_bus = self.get_cr(field!(opcode >> 9, ControlRegister));", 0),
        "f7->right" => ("let right_bus = sign_extend_field(opcode >> 3, 8);", 0),

        "right->addr_base" => ("let addr_base_bus = right_bus;", 1),
        "left->mem_data" => ("let mem_data = left_bus;", 1),

        "alu_add->result" => ("let result_bus = left_bus.wrapping_add(right_bus);", 1),
        "alu_and->result" => ("let result_bus = left_bus & right_bus;", 1),
        "alu_or->result" => ("let result_bus = left_bus | right_bus;", 1),
        "alu_xor->result" => ("let result_bus = left_bus ^ right_bus;", 1),
        "alu_sub->result" => ("let result_bus = left_bus.wrapping_sub(right_bus);", 1),
        "alu_upsample->result" => ("let result_bus = (left_bus & 0xff) | (right_bus & 0xff) << 8;", 1),

        "f8->addr_offset" => ("let mem_address = addr_base_bus.wrapping_add(sign_extend_field(opcode >> 3, 7));", 2),
        "zero->addr_offset" => ("let mem_address = addr_base_bus;", 2),
        "one->addr_offset" => ("let mem_address = addr_base_bus.wrapping_add(1);", 2),

        "mem_address->pc" => ("self.pc = mem_address;", 3),
        "read_mem_data" => ("let mem_data = self.read_memory_virt(&VirtualMemoryAddress::from(mem_address), &segment)?;", 3),
        "write_mem_data" => ("self.write_memory_virt(&VirtualMemoryAddress::from(mem_address), &segment, left_bus)?;", 3),

        "mem_data->instruction" => ("self.next_instruction = mem_data;", 4),
        "mem_data->result" => ("let result_bus = mem_data;", 4),

        "result->f1" => ("self.set_gpr(field!(opcode, Gpr), result_bus);", 5),
        "result->f6" => ("self.set_cr(field!(opcode >> 9, ControlRegister), result_bus);", 5),

        "end_instruction" => ("self.current_instruction = self.next_instruction;", 0),

        _ => return Err(CodegenError::UnknownMicroinstruction(microinstruction.to_owned()))
    };
    Ok((format!("{} // {}", code, microinstruction), priority))
}

#[derive(Debug, Error)]
enum CodegenError {
    #[error("Bad substitution: {0}")]
    BadSubstitution(String),
    #[error("Unknown microinstruction: {0}")]
    UnknownMicroinstruction(String),
}
