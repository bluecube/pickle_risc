use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Write;
use std::iter::repeat;
use std::ops::Range;
use std::path::Path;

use instruction_set::{
    Instruction, InstructionEncodingArgType, InstructionEncodingPiece, InstructionSet,
    INSTRUCTION_BITS, OPCODE_BITS,
};
use itertools::Itertools;

fn main() {
    generate_code().unwrap();
}

fn generate_code() -> anyhow::Result<()> {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let instruction_target_path = Path::new(&out_dir).join("instruction_def.rs");
    let microcode_target_path = Path::new(&out_dir).join("microcode_def.rs");
    let parse_asm_match_target_path = Path::new(&out_dir).join("parse_asm_match.rs");
    let definition_path = Path::new("..").join("instruction_set.json5");

    //println!("cargo:warning={}", instruction_target_path.to_str().unwrap());
    //println!("cargo:warning={}", microcode_target_path.to_str().unwrap());
    println!(
        "cargo:warning={}",
        parse_asm_match_target_path.to_str().unwrap()
    );
    println!(
        "cargo:rerun-if-changed={}",
        definition_path.to_str().unwrap()
    );

    let definition = InstructionSet::load(definition_path)?;
    let mut instruction_target = File::create(instruction_target_path)?;
    let mut microcode_target = File::create(microcode_target_path)?;
    let mut parse_asm_match_target = File::create(parse_asm_match_target_path)?;

    generate_instruction(&definition, &mut instruction_target)?;
    generate_opcode(&definition, &mut instruction_target)?;
    generate_microcode_match(&definition, &mut microcode_target)?;
    generate_parse_asm_match(&definition, &mut parse_asm_match_target)?;

    Ok(())
}

fn generate_instruction(definition: &InstructionSet, target: &mut File) -> anyhow::Result<()> {
    writeln!(target, "#[derive(Debug, Copy, Clone, Eq, PartialEq)]")?;
    writeln!(target, "#[cfg_attr(test, derive(Arbitrary))]")?;

    writeln!(target, "pub enum Instruction {{")?;
    for (mnemonic, instruction_def) in &definition.instructions {
        generate_opcode_line(mnemonic, instruction_def, true, target)?;
    }
    writeln!(target, "}}")?;

    writeln!(target, "impl std::fmt::Display for Instruction {{")?;
    writeln!(
        target,
        "    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {{"
    )?;
    writeln!(target, "        match self {{")?;
    for (mnemonic, instruction_def) in &definition.instructions {
        generate_instruction_display_arm(mnemonic, instruction_def, target)?;
    }
    writeln!(target, "        }}")?;
    writeln!(target, "    }}")?;
    writeln!(target, "}}")?;

    writeln!(target, "impl TryFrom<Word> for Instruction {{")?;
    writeln!(target, "    type Error = <Opcode as TryFrom<Word>>::Error;")?;
    writeln!(
        target,
        "    fn try_from(v: Word) -> Result<Self, Self::Error> {{"
    )?;
    writeln!(target, "        Ok(match Opcode::try_from(v)? {{")?;
    for (mnemonic, instruction_def) in &definition.instructions {
        generate_instruction_match_arm(mnemonic, instruction_def, target)?;
    }
    writeln!(target, "        }})")?;
    writeln!(target, "    }}")?;
    writeln!(target, "}}")?;

    writeln!(target, "impl From<Instruction> for Word {{")?;
    writeln!(target, "    fn from(i: Instruction) -> Word {{")?;
    writeln!(target, "        match i {{")?;
    for (mnemonic, instruction_def) in &definition.instructions {
        generate_instruction_output_arm(mnemonic, instruction_def, target)?;
    }
    writeln!(target, "        }}")?;
    writeln!(target, "    }}")?;
    writeln!(target, "}}")?;
    Ok(())
}

fn generate_opcode(definition: &InstructionSet, target: &mut File) -> anyhow::Result<()> {
    writeln!(target, "#[derive(Debug, Copy, Clone, Eq, PartialEq)]")?;
    writeln!(target, "pub enum Opcode {{")?;
    for (mnemonic, instruction_def) in &definition.instructions {
        generate_opcode_line(mnemonic, instruction_def, false, target)?;
    }
    writeln!(target, "}}")?;

    writeln!(target, "impl TryFrom<Word> for Opcode {{")?;
    writeln!(target, "    type Error = InvalidInstructionError;")?;
    writeln!(
        target,
        "    fn try_from(v: Word) -> Result<Self, Self::Error> {{"
    )?;
    writeln!(
        target,
        "        match (v >> {}) & {} {{",
        INSTRUCTION_BITS - OPCODE_BITS,
        (1 << OPCODE_BITS) - 1,
    )?;
    let opcode_table = make_opcode_table(definition)?;
    for (count, (first_opcode, instruction)) in opcode_table
        .iter()
        .enumerate()
        .dedup_by_with_count(|x, y| x.1.map(|x| x.0) == y.1.map(|x| x.0))
    {
        generate_opcode_match_arm(instruction, first_opcode..(first_opcode + count), target)?;
    }
    writeln!(
        target,
        "            {:#04x} ..= u16::MAX => unreachable!(\"Opcode should be limited by the bitmask\"),",
        1 << OPCODE_BITS
    )?;
    writeln!(target, "        }}")?;
    writeln!(target, "    }}")?;
    writeln!(target, "}}")?;

    Ok(())
}

fn generate_instruction_display_arm(
    mnemonic: &str,
    instruction_def: &Instruction,
    target: &mut File,
) -> anyhow::Result<()> {
    write!(
        target,
        "            Instruction::{}",
        mnemonic_to_cammel_case(mnemonic)
    )?;

    if !instruction_def.args.is_empty() {
        write!(target, " {{ ",)?;
        for (arg, _) in &instruction_def.args {
            write!(target, "{}, ", arg)?;
        }
        write!(target, "}}")?;
    }
    write!(target, " => write!(f, \"{}", mnemonic)?;

    let mut first = true;
    for _ in 0..instruction_def.args.len() {
        if first {
            write!(target, " ")?;
        } else {
            write!(target, ", ")?;
        }
        write!(target, "{{}}")?;
        first = false;
    }
    write!(target, "\"")?;
    for (arg, _) in &instruction_def.args {
        write!(target, ", {}", arg)?;
    }
    writeln!(target, "),")?;
    Ok(())
}

fn generate_opcode_line(
    mnemonic: &str,
    instruction_def: &Instruction,
    with_params: bool,
    target: &mut File,
) -> anyhow::Result<()> {
    write!(target, "    {}", mnemonic_to_cammel_case(mnemonic))?;
    if with_params && !instruction_def.args.is_empty() {
        write!(target, " {{ ")?;
        for (arg, arg_type) in &instruction_def.args {
            write!(target, "{}: {}, ", arg, arg_type_to_rust(arg_type))?;
        }
        write!(target, "}}")?;
    }
    writeln!(target, ",")?;
    Ok(())
}

fn mnemonic_to_cammel_case(mnemonic: &str) -> String {
    let mut ret = String::with_capacity(mnemonic.len());
    let mut at_boundary = true;
    for c in mnemonic.chars() {
        if c == '_' {
            at_boundary = true;
        } else if at_boundary {
            at_boundary = false;
            ret.extend(c.to_uppercase());
        } else {
            ret.push(c);
        }
    }
    ret
}

fn arg_type_to_rust(t: &InstructionEncodingArgType) -> String {
    match t {
        InstructionEncodingArgType::Gpr => "Gpr".to_string(),
        InstructionEncodingArgType::ControlRegister => "ControlRegister".to_string(),
        InstructionEncodingArgType::Immediate { signed, bits } => {
            format!("{}{}", if *signed { "i" } else { "u" }, bits)
        }
    }
}

fn make_opcode_table(
    definition: &InstructionSet,
) -> anyhow::Result<Vec<Option<(&str, &Instruction)>>> {
    let mut table: Vec<Option<(&str, &Instruction)>> =
        repeat(None).take(1 << OPCODE_BITS).collect();

    for (mnemonic, instruction_def) in &definition.instructions {
        let encoding = instruction_def.encoding(mnemonic)?;
        for opcode in expand_encoding(&encoding[..OPCODE_BITS]) {
            table[opcode] = Some((mnemonic, instruction_def));
        }
    }

    Ok(table)
}

/// Converts a str with 0, 1 and other into all numbers that match this bit string
fn expand_encoding(s: &str) -> impl Iterator<Item = usize> {
    s.chars()
        .map(|c| match c {
            '0' => 0..=0,
            '1' => 1..=1,
            _ => 0..=1,
        })
        .multi_cartesian_product()
        .map(|x| x.iter().fold(0, |acc, digit| (acc << 1) + digit))
}

fn generate_opcode_match_arm(
    instruction: &Option<(&str, &Instruction)>,
    opcodes: Range<usize>,
    target: &mut File,
) -> anyhow::Result<()> {
    write!(target, "            ")?;
    if opcodes.len() == 1 {
        write!(target, "{:#04x} => ", opcodes.start)?;
    } else {
        write!(
            target,
            "{:#04x} ..= {:#04x} => ",
            opcodes.start,
            opcodes.end - 1
        )?;
    }
    match instruction {
        Some((mnemonic, _)) => {
            write!(target, "Ok(Opcode::{}", mnemonic_to_cammel_case(mnemonic))?;
            writeln!(target, "),")?;
        }
        None => {
            writeln!(target, "Err(InvalidInstructionError::InvalidOpcode(v)),")?;
        }
    };
    Ok(())
}

fn generate_instruction_match_arm(
    mnemonic: &str,
    instruction_def: &Instruction,
    target: &mut File,
) -> anyhow::Result<()> {
    let converted_mnemonic = mnemonic_to_cammel_case(mnemonic);

    write!(
        target,
        "            Opcode::{} => Instruction::{}",
        converted_mnemonic, converted_mnemonic
    )?;

    // for each arg of this instruction alculate the index of the rightmost bit
    // in the arg encoding
    let mut arg_offsets: HashMap<String, usize> = HashMap::new();
    let mut offset = 0;
    for encoding_piece in &instruction_def.encoding_pieces {
        match &encoding_piece {
            InstructionEncodingPiece::Literal(s) => offset += s.len(),
            InstructionEncodingPiece::Ignored(l) => offset += l,
            InstructionEncodingPiece::Arg(arg_name) => {
                offset += instruction_def.args[arg_name].bits();
                arg_offsets.insert(arg_name.to_string(), offset);
            }
        }
    }

    if !instruction_def.args.is_empty() {
        writeln!(target, " {{",)?;
        for (arg_name, arg_type) in &instruction_def.args {
            write!(target, "                {}: ", arg_name)?;
            if let InstructionEncodingArgType::Immediate {
                signed: true,
                bits: _,
            } = arg_type
            {
                write!(target, "sign_extend_")?;
            }
            write!(target, "field(")?;
            let shift = INSTRUCTION_BITS - arg_offsets[arg_name];
            if shift == 0 {
                write!(target, "v")?;
            } else {
                write!(target, "v >> {}", shift)?;
            }
            writeln!(target, ", {}),", instruction_def.args[arg_name].bits())?;
        }
        write!(target, "            }}")?;
    }
    writeln!(target, ",")?;

    Ok(())
}

fn generate_instruction_output_arm(
    mnemonic: &str,
    instruction_def: &Instruction,
    target: &mut File,
) -> anyhow::Result<()> {
    let converted_mnemonic = mnemonic_to_cammel_case(mnemonic);

    write!(target, "            Instruction::{}", converted_mnemonic)?;

    if !instruction_def.args.is_empty() {
        write!(target, " {{ ",)?;
        for (arg_name, _arg_type) in &instruction_def.args {
            write!(target, "{}, ", arg_name)?;
        }
        write!(target, "}}")?;
    }

    writeln!(target, " => {{")?;
    write!(target, "                ")?;

    let mut fixed_encoding_part = 0;
    let mut offset = INSTRUCTION_BITS;
    for encoding_piece in &instruction_def.encoding_pieces {
        match &encoding_piece {
            InstructionEncodingPiece::Literal(s) => {
                offset -= s.len();
                fixed_encoding_part |= u16::from_str_radix(&s, 2).unwrap() << offset;
            }
            InstructionEncodingPiece::Ignored(l) => offset -= l,
            InstructionEncodingPiece::Arg(arg_name) => {
                let bits = instruction_def.args[arg_name].bits();
                offset -= bits;

                if offset > 0 {
                    write!(target, "(")?;
                }

                if let InstructionEncodingArgType::Immediate { signed: true, bits } =
                    instruction_def.args[arg_name]
                {
                    write!(target, "encode_signed_field({arg_name}, {bits})")?;
                } else {
                    write!(target, "u16::from({arg_name})")?;
                }

                if offset > 0 {
                    write!(target, " << {offset})")?;
                }
                write!(target, " | ")?;
            }
        }
    }
    write!(target, "{:#06x}u16", fixed_encoding_part)?;
    writeln!(target)?;

    writeln!(target, "            }},")?;

    Ok(())
}

fn generate_microcode_match(definition: &InstructionSet, target: &mut File) -> anyhow::Result<()> {
    writeln!(
        target,
        "match (Opcode::try_from(self.current_instruction), u8::from(self.step)) {{"
    )?;
    for (mnemonic, instruction_def) in &definition.instructions {
        generate_microcode_match_arm(
            format!("Ok(Opcode::{})", mnemonic_to_cammel_case(mnemonic)).as_ref(),
            instruction_def.microcode.as_ref(),
            &definition.substitutions,
            target,
        )?;
    }
    generate_microcode_match_arm(
        "Err(_)",
        definition.invalid_instruction_microcode.as_ref(),
        &definition.substitutions,
        target,
    )?;
    writeln!(target, "}}")?;
    Ok(())
}

fn generate_microcode_match_arm(
    opcode: &str,
    microcode: Option<&Vec<Vec<String>>>,
    substitutions: &HashMap<String, Vec<String>>,
    target: &mut File,
) -> anyhow::Result<()> {
    if let Some(microcode) = microcode {
        for (i, microcode_step) in microcode.iter().enumerate() {
            generate_microcode_step(opcode, i, microcode_step, substitutions, target)?;
        }
    }
    writeln!(target, "    ({}, _) => {{", opcode)?;
    if let Some(microcode) = microcode {
        writeln!(
            target,
            "        unreachable!(\"The instruction should only have {} steps\");",
            microcode.len()
        )?;
    } else {
        writeln!(target, "        todo!(\"Missing microcode\");")?;
    }
    writeln!(target, "    }},")?;
    Ok(())
}

fn generate_microcode_step(
    opcode: &str,
    step: usize,
    microcode: &[String],
    substitutions: &HashMap<String, Vec<String>>,
    target: &mut File,
) -> anyhow::Result<()> {
    writeln!(target, "    ({}, {}) => {{", opcode, step)?;
    writeln!(
        target,
        "        #[allow(unused_mut,unused_variables,unused_assignments)] let mut segment = VirtualMemorySegment::Data;",
    )?;

    microcode
        .iter()
        .flat_map(|microinstruction| substitute_microinstruction(microinstruction, substitutions))
        .map(translate_microinstruction)
        .sorted_by_key(|(_code, phase)| *phase)
        .try_for_each(|(code, _phase)| writeln!(target, "        {}", code))?;

    // TODO: Conditionals in microcode
    writeln!(target, "    }},")?;

    Ok(())
}

fn substitute_microinstruction<'a>(
    microinstruction: &'a str,
    substitutions: &'a HashMap<String, Vec<String>>,
) -> impl Iterator<Item = &'a str> {
    if let Some(susbst_name) = microinstruction.strip_prefix('$') {
        if let Some(subst) = substitutions.get(susbst_name) {
            either::Left(subst.iter().map(|x| x.as_str()))
        } else {
            panic!("Bad substitution: {:?}", microinstruction);
        }
    } else {
        either::Right(std::iter::once(microinstruction))
    }
}

/// Parse a microinstruction, returns rust code to emulate it, and its phase
/// (to produce microinstructions that produce value before the ones that consume them)
fn translate_microinstruction(microinstruction: &str) -> (String, usize) {
    let (code, priority) = match microinstruction {
        "pc->left" => ("let left_bus = self.pc;", 0),
        "pc->addr_base" => ("let addr_base_bus = self.pc;", 0),
        "zero->left" => ("let left_bus = 0;", 0),
        "f2->left" => ("let left_bus = self.get_gpr(field(opcode, 3));", 0),
        "f3->left" => ("let left_bus = self.get_gpr(field(opcode >> 3, 3));", 0),
        "f4->right" => ("let right_bus = self.get_gpr(field(opcode >> 6, 3));", 0),
        "f5->right" => ("let right_bus = self.get_gpr(field(opcode >> 10, 3));", 0),
        "f6->right" => ("let right_bus = self.get_cr(field(opcode >> 9, 3));", 0),
        "f7->right" => ("let right_bus: Word = sign_extend_field(opcode >> 3, 8);", 0),

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
        "program_segment" => ("segment = VirtualMemorySegment::Program;", 2),

        "mem_address->pc" => ("self.pc = mem_address;", 3),
        "read_mem_data" => ("let mem_data = self.read_memory(&VirtualMemoryAddress::from(mem_address), &segment, memory)?;", 3),
        "write_mem_data" => ("self.write_memory_virt(&VirtualMemoryAddress::from(mem_address), &segment, memory, left_bus)?;", 3),

        "mem_data->instruction" => ("self.next_instruction = mem_data;", 4),
        "mem_data->result" => ("let result_bus = mem_data;", 4),

        "result->f1" => ("self.set_gpr(field(opcode, 3), result_bus);", 5),
        "result->f6" => ("self.set_cr(field(opcode >> 9, 3), result_bus);", 5),

        "end_instruction" => ("self.end_instruction();", 6),
        "break" => ("Err(EmulatorError::Break)?;", 7),

        _ => todo!("Unknown microinstruction: {:?}", microinstruction)
    };
    (format!("{} // {}", code, microinstruction), priority)
}

fn generate_parse_asm_match(definition: &InstructionSet, target: &mut File) -> anyhow::Result<()> {
    writeln!(target, "match mnemonic {{")?;
    for (mnemonic, instruction_def) in &definition.instructions {
        generate_opcode_parse_match_arm(mnemonic, instruction_def, target)?;
    }
    writeln!(target, "    _ => None,")?;
    writeln!(target, "}}")?;

    Ok(())
}

fn generate_opcode_parse_match_arm(
    mnemonic: &str,
    instruction_def: &Instruction,
    target: &mut File,
) -> anyhow::Result<()> {
    writeln!(target, "    {mnemonic:?} => {{")?;
    let mut first = true;
    for (arg_name, arg_type) in &instruction_def.args {
        if !first {
            writeln!(target, "        one_token(tokens, Token::Comma)?;")?;
        }
        first = false;
        write!(target, "        let {arg_name} = ")?;
        match arg_type {
            InstructionEncodingArgType::Gpr => write!(target, "gpr(tokens)")?,
            InstructionEncodingArgType::ControlRegister => write!(target, "cr(tokens)")?,
            InstructionEncodingArgType::Immediate { signed, bits } => {
                let signed_char = if *signed { 'i' } else { 'u' };
                let intermediate_bis = ((bits + 7) / 8).next_power_of_two() * 8;
                write!(target, "immediate::<{signed_char}{intermediate_bis}, {signed_char}{bits}>(state, tokens)")?
            }
        }
        writeln!(target, "?;")?;
    }

    let mnemonic_cammel_case = mnemonic_to_cammel_case(mnemonic);
    if instruction_def.args.is_empty() {
        writeln!(target, "        Some(Instruction::{mnemonic_cammel_case})")?;
    } else {
        writeln!(
            target,
            "        Some(Instruction::{mnemonic_cammel_case} {{"
        )?;
        for (arg_name, _) in &instruction_def.args {
            writeln!(target, "            {arg_name},")?;
        }
        writeln!(target, "        }})")?;
    }
    writeln!(target, "    }},")?;

    Ok(())
}
