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
    let target_path = Path::new(&out_dir).join("instruction_def.rs");
    let definition_path = Path::new("..").join("..").join("instruction_set.json5");

    println!(
        "cargo:warning=Output goes to {}",
        target_path.to_str().unwrap()
    );
    println!(
        "cargo:rerun-if-changed={}",
        definition_path.to_str().unwrap()
    );

    let definition = InstructionSet::load(definition_path)?;
    let mut target = File::create(target_path)?;

    generate_instruction(&definition, &mut target)?;
    generate_opcode(&definition, &mut target)?;

    Ok(())
}

fn generate_instruction(definition: &InstructionSet, target: &mut File) -> anyhow::Result<()> {
    writeln!(target, "#[derive(Debug, Copy, Clone, Eq, PartialEq)]")?;
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
        "        match v >> {} {{",
        INSTRUCTION_BITS - OPCODE_BITS
    )?;
    let opcode_table = make_opcode_table(definition)?;
    for (count, (first_opcode, instruction)) in opcode_table
        .iter()
        .enumerate()
        .dedup_by_with_count(|x, y| x.1.map(|x| x.0) == y.1.map(|x| x.0))
    {
        generate_opcode_match_arm(instruction, first_opcode..(first_opcode + count), target)?;
    }
    writeln!(target, "            _ => unreachable!(),")?;
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
        writeln!(target, "{:#04x} =>", opcodes.start)?;
    } else {
        writeln!(
            target,
            "{:#04x}..={:#04x} =>",
            opcodes.start,
            opcodes.end - 1
        )?;
    }
    write!(target, "                ")?;
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
