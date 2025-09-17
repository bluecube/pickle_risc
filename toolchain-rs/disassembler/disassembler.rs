use std::fmt::Display;
use toolchain_core::instruction::Instruction;

#[derive(Clone, Copy, Debug)]
pub struct Disassembler<'a> {
    data: &'a [u16],
    offset: u16,
    next_is_immediate: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Item {
    address: u16,
    content: ItemContent,
}

#[derive(Clone, Copy, Debug)]
pub enum ItemContent {
    Instruction(Instruction),
    Immediate(u16),
    InvalidInstruction,
}

impl<'a> Disassembler<'a> {
    pub fn new(data: &'a [u16]) -> Self {
        Disassembler {
            data,
            offset: 0,
            next_is_immediate: false,
        }
    }
}

impl<'a> Iterator for Disassembler<'a> {
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        let (first, rest) = self.data.split_first()?;
        let address = self.offset;

        self.data = rest;
        self.offset += 1;

        Some(Item {
            address,
            content: if self.next_is_immediate {
                self.next_is_immediate = false;
                ItemContent::Immediate(*first)
            } else {
                if let Some(instruction) = Instruction::decode(*first) {
                    self.next_is_immediate = match instruction {
                        Instruction::Ldi { rd: _ } => true,
                        _ => false,
                    };
                    ItemContent::Instruction(instruction)
                } else {
                    self.next_is_immediate = false;
                    ItemContent::InvalidInstruction
                }
            },
        })
    }
}

impl Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#06x}: ", self.address)?;
        match self.content {
            ItemContent::Instruction(instruction) => write!(f, "{:?}", instruction), // TODO: Use Display, once it is implemented
            ItemContent::Immediate(v) => write!(f, "    0x{:x}", v),
            ItemContent::InvalidInstruction => write!(f, "<invalid instruction>"),
        }
    }
}
