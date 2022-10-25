use itertools::Itertools;
use iset::IntervalMap;
use ux::*; // Non-standard integer types

use crate::util::*;
use crate::cpu_types::*;

#[derive(Debug)]
pub struct CpuState {
    gpr: [Word; 7],

    pc: Word,

    alu_flags: Word,
    context_id: ContextId,
    int_c: Word,
    int_cause: Word,
    mmu_addr: Word,

    current_instruction: Word,
    next_instruction: Word,

    page_table: [PageTableRecord; 1 << PageTableIndex::BITS],

    physical_memory: IntervalMap<u32, Box<dyn MemoryMapping>>,
}

impl CpuState {
    pub fn get_gpr(&self, index: Gpr) -> Word {
        if index == Gpr::new(0) {
            0
        } else {
            self.gpr[usize::try_from(index).unwrap() - 1]
        }
    }

    pub fn set_gpr(&mut self, index: Gpr, value: Word) {
        if index > Gpr::new(0) {
            self.gpr[usize::try_from(index).unwrap() - 1] = value
        }
    }

    pub fn get_cr(&self, index: ControlRegister) -> Word {
        todo!();
    }

    pub fn set_cr(&mut self, index: ControlRegister, value: Word) {
        todo!();
    }

    fn read_memory_virt(&self, address: &VirtualMemoryAddress, segment: &VirtualMemorySegment) -> anyhow::Result<Word> {
        if let Some(physical_address) = self.map_memory(address, segment, false) {
            self.read_memory_phys(&physical_address)
        } else {
            todo!("Interrupt");
        }
    }

    fn write_memory_virt(&mut self, address: &VirtualMemoryAddress, segment: &VirtualMemorySegment, value: Word) -> anyhow::Result<()> {
        if let Some(physical_address) = self.map_memory(address, segment, true) {
            self.write_memory_phys(&physical_address, value)
        } else {
            todo!("Interrupt");
        }
    }

    fn read_memory_phys(&self, address: &PhysicalMemoryAddress) -> anyhow::Result<Word> {
        let a = u32::from(u24::from(address));
        let (mapping_range, mapping) = self.physical_memory
            .overlap(a)
            .at_most_one()
            .expect("Memory mappings should not overlap")
            .ok_or_else(|| EmulatorError::NonMappedPhysicalMemory{ address: *address, pc: self.pc })?;

        mapping.read(a - mapping_range.start)
    }

    fn write_memory_phys(&mut self, address: &PhysicalMemoryAddress, value: Word) -> anyhow::Result<()> {
        let a = u32::from(u24::from(address));
        let (mapping_range, mapping) = self.physical_memory
            .overlap_mut(a)
            .at_most_one()
            .expect("Memory mappings should not overlap")
            .ok_or_else(|| EmulatorError::NonMappedPhysicalMemory{ address: *address, pc: self.pc })?;

        mapping.write(a - mapping_range.start, value)
    }

    fn map_memory(&self, address: &VirtualMemoryAddress, segment: &VirtualMemorySegment, write: bool) -> Option<PhysicalMemoryAddress> {
        let page_table_index = PageTableIndex {
            context_id: self.context_id,
            segment: *segment,
            page_number: address.page_number
        };

        let page = self.page_table[usize::from(&page_table_index)];

        if write && !page.writable {
            None
        } else if !write && !page.readable {
            None
        } else {
            Some(
                PhysicalMemoryAddress {
                    frame_number: page.frame_number,
                    offset: address.offset
                }
            )
        }
    }

    fn write_memory_mapping(&mut self, page_table_index: &PageTableIndex, record: PageTableRecord) {
        self.page_table[usize::from(page_table_index)] = record;
    }

    pub fn step(&mut self, opcode: Word) -> anyhow::Result<()> {
        include!(concat!(env!("OUT_DIR"), "/instruction_handler.rs"));
        Ok(())
    }
}

pub trait MemoryMapping: std::fmt::Debug {
    fn size(&self) -> u32;
    fn read(&self, address: u32) -> anyhow::Result<Word>;
    fn write(&mut self, address: u32, value: Word) -> anyhow::Result<()>;
}
