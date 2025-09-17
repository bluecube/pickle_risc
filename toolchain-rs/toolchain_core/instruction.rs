use num_enum::TryFromPrimitive;
#[cfg(test)]
use proptest::{
    arbitrary::Arbitrary,
    strategy::{BoxedStrategy, Strategy},
};
use std::str::FromStr;
use strum::EnumString;
#[cfg(test)]
use test_strategy::Arbitrary;
use thiserror::Error;
use ux::*; // Non-standard integer types

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Reg(u16);

impl std::fmt::Display for Reg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "r{}", self.0)
    }
}

impl TryFrom<u16> for Reg {
    type Error = ();
    fn try_from(v: u16) -> Result<Self, Self::Error> {
        if v < 16 {
            Ok(Reg(v))
        } else {
            Err(())
        }
    }
}

impl From<Reg> for u16 {
    fn from(r: Reg) -> u16 {
        r.0
    }
}

impl From<&Reg> for u16 {
    fn from(r: &Reg) -> u16 {
        r.0
    }
}

impl From<u4> for Reg {
    fn from(value: u4) -> Self {
        Reg(value.into())
    }
}

impl Reg {
    /// Create a new register index
    pub fn new(i: u16) -> Result<Reg, ()> {
        i.try_into()
    }
}

impl FromStr for Reg {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.strip_prefix("r")
            .and_then(|suffix| suffix.parse::<u16>().ok())
            .and_then(|n| Reg::try_from(n).ok())
            .ok_or(())
    }
}

#[cfg(test)]
impl Arbitrary for Reg {
    type Parameters = ();
    type Strategy = BoxedStrategy<Reg>;

    fn arbitrary_with(_: ()) -> Self::Strategy {
        (0u16..16u16).prop_map(|x| Reg(x)).boxed()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, EnumString)]
#[cfg_attr(test, derive(Arbitrary))]
#[repr(u16)]
pub enum ControlRegister {
    Display = 0,
    CpuStatus = 1,
    ContextID = 2,
    IntCause = 3,
    IntBase = 4,
    IntPc = 5,
    MMUAddr = 6,
    MMUData = 7,
}

impl From<ControlRegister> for u16 {
    fn from(cr: ControlRegister) -> u16 {
        cr as u16
    }
}

impl From<&ControlRegister> for u16 {
    fn from(cr: &ControlRegister) -> u16 {
        *cr as u16
    }
}

impl std::fmt::Display for ControlRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

#[derive(Copy, Clone, Debug, Error, PartialEq, Eq)]
pub enum InvalidInstructionError {
    #[error("Invalid instruction {0:#06x}")]
    InvalidOpcode(u16),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
#[allow(non_camel_case_types)]
pub enum Instruction {
    Add { rd: Reg, ra: Reg, rb: Reg },
    Sub { rd: Reg, ra: Reg, rb: Reg },
    And { rd: Reg, ra: Reg, rb: Reg },
    Or { rd: Reg, ra: Reg, rb: Reg },
    Xor { rd: Reg, ra: Reg, rb: Reg },
    Pack { rd: Reg, ra: Reg, rb: Reg },
    Bcmp { rd: Reg, ra: Reg, rb: Reg },
    Jalr { rd: Reg, offset: i8 },
    Addi { rd: Reg, v: i8 },
    Bz { rd: Reg, offset: i8 },
    Bnz { rd: Reg, offset: i8 },
    Ld { rd: Reg, addr: Reg, offset: i4 },
    St { val: Reg, addr: Reg, offset: i4 },
    Bc { offset: i8 },
    Bnc { offset: i8 },
    Addc { rd: Reg, rb: Reg },
    Subc { rd: Reg, rb: Reg },
    Add_c { rd: Reg, rb: Reg },
    Shr { rd: Reg, rb: Reg },
    Shrc { rd: Reg, rb: Reg },
    Shra { rd: Reg, rb: Reg },
    Shr8 { rd: Reg, rb: Reg },
    Jal { rd: Reg, addr: Reg },
    Ldp { rd: Reg, addr: Reg },
    Ldi { rd: Reg },
    St_c { rd: Reg, addr: Reg },
    Andi { rd: Reg, v: i4 },
    Ori { rd: Reg, v: i4 },
    Xori { rd: Reg, v: i4 },
    Ldcr { rd: Reg, cr: ControlRegister },
    Stcr { val: Reg, cr: ControlRegister },
    Syscall { val: u4 },
    Reti,
    Break,
}

impl Instruction {
    pub fn encode(&self) -> u16 {
        match self {
            Instruction::Add { rd, ra, rb } => encode_rrr(rd, rb, ra, 0b0000),
            Instruction::Sub { rd, ra, rb } => encode_rrr(rd, rb, ra, 0b0001),
            Instruction::And { rd, ra, rb } => encode_rrr(rd, rb, ra, 0b0010),
            Instruction::Or { rd, ra, rb } => encode_rrr(rd, rb, ra, 0b0011),
            Instruction::Xor { rd, ra, rb } => encode_rrr(rd, rb, ra, 0b0100),
            Instruction::Pack { rd, ra, rb } => encode_rrr(rd, rb, ra, 0b0101),
            Instruction::Bcmp { rd, ra, rb } => encode_rrr(rd, rb, ra, 0b0110),
            Instruction::Jalr { rd, offset } => encode_r8(rd, offset, 0b0111),
            // Unused 0b1000
            Instruction::Addi { rd, v } => encode_r8(rd, v, 0b1001),
            Instruction::Bz { rd, offset } => encode_r8(rd, offset, 0b1010),
            Instruction::Bnz { rd, offset } => encode_r8(rd, offset, 0b1011),
            Instruction::Ld { rd, addr, offset } => encode_rr4(rd, addr, offset, 0b1100),
            Instruction::St { val, addr, offset } => encode_rr4(val, addr, offset, 0b1101),
            Instruction::Bc { offset } => encode_8(offset, 0b0000, 0b1110),
            Instruction::Bnc { offset } => encode_8(offset, 0b0001, 0b1110),
            Instruction::Addc { rd, rb } => encode_rr(rd, rb, 0b0010, 0b1110),
            Instruction::Subc { rd, rb } => encode_rr(rd, rb, 0b0011, 0b1110),
            Instruction::Add_c { rd, rb } => encode_rr(rd, rb, 0b0100, 0b1110),
            Instruction::Shr { rd, rb } => encode_rr(rd, rb, 0b0101, 0b1110),
            Instruction::Shrc { rd, rb } => encode_rr(rd, rb, 0b0110, 0b1110),
            Instruction::Shra { rd, rb } => encode_rr(rd, rb, 0b0111, 0b1110),
            Instruction::Shr8 { rd, rb } => encode_rr(rd, rb, 0b1000, 0b1110),
            Instruction::Jal { rd, addr } => encode_rr(rd, addr, 0b1001, 0b1110),
            Instruction::Ldp { rd, addr } => encode_rr(rd, addr, 0b1010, 0b1110),
            Instruction::Ldi { rd } => encode_r4(rd, &(0i8.try_into().unwrap()), 0b1011, 0b1110),
            Instruction::St_c { rd, addr } => encode_rr(rd, addr, 0b1100, 0b1110),
            Instruction::Andi { rd, v } => encode_r4(rd, v, 0b1101, 0b1110),
            Instruction::Ori { rd, v } => encode_r4(rd, v, 0b1110, 0b1110),
            Instruction::Xori { rd, v } => encode_r4(rd, v, 0b1111, 0b1110),
            Instruction::Ldcr { rd, cr } => encode_r4u(rd, cr.into(), 0b0000, 0b1111),
            Instruction::Stcr { val, cr } => encode_r4u(val, cr.into(), 0b0001, 0b1111),
            Instruction::Syscall { val } => {
                encode_r4u(&Reg::new(0).unwrap(), (*val).into(), 0b0010, 0b1111)
            }
            Instruction::Reti => 0b00111111,
            // unused 0b01001111 - 0b11101111
            Instruction::Break => 0b11111111,
        }
    }

    pub fn decode(word: u16) -> Option<Self> {
        let (d, b, a, opcode_l) = to_nibbles(word);
        let opcode_l = u8::from(opcode_l);
        let rd = d.into();
        let rb = b.into();
        let ra = a.into();

        let imm8ba: i8 = (u8::from(a) << 4 | u8::from(b)) as i8;
        let imm4a = u4_bits_to_i4(a);

        Some(match opcode_l {
            0b0000 => Self::Add { rd, ra, rb },
            0b0001 => Self::Sub { rd, ra, rb },
            0b0010 => Self::And { rd, ra, rb },
            0b0011 => Self::Or { rd, ra, rb },
            0b0100 => Self::Xor { rd, ra, rb },
            0b0101 => Self::Pack { rd, ra, rb },
            0b0110 => Self::Bcmp { rd, ra, rb },
            0b0111 => Self::Jalr { rd, offset: imm8ba },
            0b1000 => return None,
            0b1001 => Self::Addi { rd, v: imm8ba },
            0b1010 => Self::Bz { rd, offset: imm8ba },
            0b1011 => Self::Bnz { rd, offset: imm8ba },
            0b1100 => Self::Ld {
                rd,
                addr: rb,
                offset: imm4a,
            },
            0b1101 => Self::St {
                val: rd,
                addr: rb,
                offset: imm4a,
            },
            0b1110 => {
                let opcode_h = u8::from(a);
                let imm8bd: i8 = (u8::from(d) << 4 | u8::from(b)) as i8;
                let imm4b = u4_bits_to_i4(b);
                match opcode_h {
                    0b0000 => Self::Bc { offset: imm8bd },
                    0b0001 => Self::Bnc { offset: imm8bd },
                    0b0010 => Self::Addc { rd, rb },
                    0b0011 => Self::Subc { rd, rb },
                    0b0100 => Self::Add_c { rd, rb },
                    0b0101 => Self::Shr { rd, rb },
                    0b0110 => Self::Shrc { rd, rb },
                    0b0111 => Self::Shra { rd, rb },
                    0b1000 => Self::Shr8 { rd, rb },
                    0b1001 => Self::Jal { rd, addr: rb },
                    0b1010 => Self::Ldp { rd, addr: rb },
                    0b1011 => Self::Ldi { rd },
                    0b1100 => Self::St_c { rd, addr: rb },
                    0b1101 => Self::Andi { rd, v: imm4b },
                    0b1110 => Self::Ori { rd, v: imm4b },
                    0b1111 => Self::Xori { rd, v: imm4b },
                    _ => unreachable!(),
                }
            }
            0b1111 => {
                let opcode_h = u8::from(a);
                match opcode_h {
                    0b0000 => Self::Ldcr {
                        rd,
                        cr: ControlRegister::try_from_primitive(b.into()).unwrap(),
                    },
                    0b0001 => Self::Stcr {
                        val: rd,
                        cr: ControlRegister::try_from_primitive(b.into()).unwrap(),
                    },
                    0b0010 => Self::Syscall { val: b },
                    0b0011 => Self::Reti,
                    0b0100..=0b1110 => return None,
                    0b1111 => Self::Break,
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        })
    }
}

fn encode_rrr(rd: &Reg, rb: &Reg, ra: &Reg, opcode: u16) -> u16 {
    u16::from(rd) << 12 | u16::from(rb) << 8 | u16::from(ra) << 4 | opcode
}

fn encode_r8(rd: &Reg, v: &i8, opcode: u16) -> u16 {
    let vv = (*v as u8) as u16;
    u16::from(rd) << 12 | (vv & 0xf) << 8 | (vv & 0xf0) | opcode
}

fn encode_rr4(rd: &Reg, rb: &Reg, v: &i4, opcode: u16) -> u16 {
    u16::from(rd) << 12 | u16::from(rb) << 8 | ((i8::from(*v) as u16) & 0xf) << 4 | opcode
}

fn encode_8(v: &i8, opcode_h: u16, opcode_l: u16) -> u16 {
    (((*v as u8) as u16) << 8) | opcode_h << 4 | opcode_l
}

fn encode_rr(rd: &Reg, rb: &Reg, opcode_h: u16, opcode_l: u16) -> u16 {
    u16::from(rd) << 12 | u16::from(rb) << 8 | opcode_h << 4 | opcode_l
}

fn encode_r4(rd: &Reg, v: &i4, opcode_h: u16, opcode_l: u16) -> u16 {
    encode_r4u(rd, ((i8::from(*v) as u16) & 0xf).into(), opcode_h, opcode_l)
}

fn encode_r4u(rd: &Reg, v: u16, opcode_h: u16, opcode_l: u16) -> u16 {
    u16::from(rd) << 12 | (v & 0xf) << 8 | opcode_h << 4 | opcode_l
}

fn to_nibbles(v: u16) -> (u4, u4, u4, u4) {
    (
        (v >> 12).try_into().unwrap(),
        ((v >> 8) & 0xf).try_into().unwrap(),
        ((v >> 4) & 0xf).try_into().unwrap(),
        (v & 0xf).try_into().unwrap(),
    )
}

fn u4_bits_to_i4(v: u4) -> i4 {
    let v: u8 = v.into();
    let signed = (v << 4) as i8 >> 4; // Sign extend
    signed.try_into().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;
    use test_strategy::proptest;

    #[test_case(0x0000, Instruction::Add {rd: Reg::new(0).unwrap(), ra:  Reg::new(0).unwrap(), rb: Reg::new(0).unwrap() }; "add_r0")]
    #[test_case(0xffff, Instruction::Break; "break_")]
    fn instruction_from_word(num: u16, expected: Instruction) {
        assert_eq!(Instruction::decode(num), Some(expected))
    }

    #[test]
    fn instruction_from_word_invalid_opcode() {
        assert_eq!(Instruction::decode(0x009fu16), None);
    }

    #[test_case(Instruction::Bc { offset: -36 }; "bc_neg36")]
    #[test_case(Instruction::Bnc { offset: 127 }; "bnc_127")]
    #[test_case(Instruction::Addi { rd: Reg(0), v: 1 }; "addi_r0_1")]
    #[test_case(Instruction::Syscall { val: 13u8.try_into().unwrap() }; "syscall_13")]
    fn instruction_word_roundtrip_example(instr: Instruction) {
        let encoded: u16 = instr.encode();
        let decoded: Instruction = Instruction::decode(encoded).unwrap();

        assert_eq!(decoded, instr);
    }

    #[proptest]
    fn instruction_word_roundtrip(instr: Instruction) {
        let encoded: u16 = instr.encode();
        let decoded: Instruction = Instruction::decode(encoded).unwrap();

        assert_eq!(decoded, instr);
    }

    #[proptest]
    fn control_register_str_roundtrip(cr: ControlRegister) {
        let string = format!("{cr}");
        let converted = ControlRegister::from_str(&string).unwrap();
        assert_eq!(cr, converted);
    }

    #[test]
    fn control_register_bad_str() {
        ControlRegister::from_str("xxxxxxxxx").unwrap_err();
    }

    #[test]
    fn control_register_bad_str_lowercase() {
        let string = format!("{}", ControlRegister::CpuStatus).to_ascii_lowercase();
        ControlRegister::from_str(&string).unwrap_err();
    }

    #[proptest]
    fn control_register_u16_roundtrip(cr: ControlRegister) {
        let num: u16 = cr.into();
        let converted = ControlRegister::try_from(num).unwrap();
        assert_eq!(cr, converted);
    }

    #[proptest]
    fn control_register_bad_u16(#[strategy(8u16..)] num: u16) {
        ControlRegister::try_from(num).unwrap_err();
    }

    #[proptest]
    fn reg_str_roundtrip(reg: Reg) {
        let string = format!("{reg}");
        let converted = Reg::from_str(&string).unwrap();
        assert_eq!(reg, converted);
    }

    #[test_case("xxxx"; "completely_wrong")]
    #[test_case("rx"; "not_a_number")]
    #[test_case("r99"; "out_of_range")]
    #[test_case("s0"; "bad_prefix")]
    fn reg_bad_str(s: &str) {
        Reg::from_str(s).unwrap_err();
    }

    #[proptest]
    fn reg_u16_roundtrip(reg: Reg) {
        let num: u16 = reg.into();
        let converted = Reg::try_from(num).unwrap();
        assert_eq!(reg, converted);
    }

    #[proptest]
    fn reg_bad_u16(#[strategy(16u16..)] num: u16) {
        Reg::try_from(num).unwrap_err();
    }
}
