use itertools::Itertools;
use iset::IntervalMap;

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

    page_table: [PageTableRecord; PageTableIndex::MAX + 1],

    physical_memory: IntervalMap<PhysicalMemoryAddress, Box<dyn MemoryMapping>>,
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

    fn read_memory_virt(&self, address: VirtualMemoryAddress) -> anyhow::Result<Word> {
        if let Some(physical_address) = self.map_memory(address, false) {
            self.read_memory_phys(physical_address)
        } else {
            todo!("Interrupt");
        }
    }

    fn write_memory_virt(&mut self, address: VirtualMemoryAddress, value: Word) -> anyhow::Result<()> {
        if let Some(physical_address) = self.map_memory(address, true) {
            self.write_memory_phys(physical_address, value)
        } else {
            todo!("Interrupt");
        }
    }

    fn read_memory_phys(&self, address: PhysicalMemoryAddress) -> anyhow::Result<Word> {
        let (mapping_range, mapping) = self.physical_memory
            .overlap(address)
            .at_most_one()
            .expect("Memory mappings should not overlap")
            .ok_or_else(|| EmulatorError::NonMappedPhysicalMemory{ address, pc: self.pc })?;

        mapping.read(address - mapping_range.start)
    }

    fn write_memory_phys(&mut self, address: PhysicalMemoryAddress, value: Word) -> anyhow::Result<()> {
        let (mapping_range, mapping) = self.physical_memory
            .overlap_mut(address)
            .at_most_one()
            .expect("Memory mappings should not overlap")
            .ok_or_else(|| EmulatorError::NonMappedPhysicalMemory{ address, pc: self.pc })?;

        mapping.write(address - mapping_range.start, value)
    }

    fn map_memory(&self, address: VirtualMemoryAddress, write: bool) -> Option<PhysicalMemoryAddress> {
        let (page_number, page_offset) = address.split_page_offset();
        let page_table_index = PageTableIndex {
            context_id: self.context_id,
            segment: address.1,
            page_number: page_number.into()
        };

        let page = self.page_table[usize::from(page_table_index)];

        if write && !page.writable {
            None
        } else if !write && !page.readable {
            None
        } else {
            Some(
                PhysicalMemoryAddress::from(page.frame_number) << PageOffset::BITS |
                PhysicalMemoryAddress::from(page_offset)
            )
        }
    }

    fn write_memory_mapping(&mut self, page_table_index: PageTableIndex, record: PageTableRecord) {
        self.page_table[usize::from(page_table_index)] = record;
    }

    pub fn step(&mut self, opcode: Word) -> anyhow::Result<()> {
        include!(concat!(env!("OUT_DIR"), "/instruction_handler.rs"));
    }
}

pub trait MemoryMapping: std::fmt::Debug {
    fn read(&self, address: PhysicalMemoryAddress) -> anyhow::Result<Word>;
    fn write(&mut self, address: PhysicalMemoryAddress, value: Word) -> anyhow::Result<()>;
}
