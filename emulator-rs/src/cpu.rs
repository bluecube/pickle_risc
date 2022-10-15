use itertools::Itertools;
use iset::IntervalMap;
use thiserror::Error;
use num_enum::{TryFromPrimitive, IntoPrimitive};
use ux::*; // Non-standard integer types


macro_rules! field {
    ($e:expr, $bits:expr) => { (($e) & ((1 << $bits) - 1)).try_into().expect("Should fit because of the mask size") }
}

type Word = u16;
type PhysicalMemoryAddress = u32;
type ContextId = u6;
type GprIndex = u3;
type CrIndex = u3;

#[derive(Copy, Clone, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u16)]
enum VirtualMemorySegment {
    DataSegment = 0,
    ProgramSegment = 1,
}

const PAGE_SIZE_BITS: u32 = 10;

#[derive(Copy, Clone, Debug)]
struct VirtualMemoryAddress(u16, VirtualMemorySegment);

impl VirtualMemoryAddress {
    /// Split address into page number and offset within the page
    fn split_page_offset(&self) -> (u6, u32) {
        (
            (self.0 >> PAGE_SIZE_BITS).try_into().expect("Should be limited by range of the input u16"),
            field!(self.0, PAGE_SIZE_BITS)
        )
    }
}

#[derive(Copy, Clone, Debug)]
struct PageTableIndex {
    context_id: u6,
    segment: VirtualMemorySegment,
    page_number: u6,
}

impl PageTableIndex {
    const BITS: u32 = 15;
    const MAX: usize = (1 << Self::BITS as usize) - 1;
}

impl From<PageTableIndex> for usize {
    /// Converting PageTableIndex into the actual usize used for indexing the table
    fn from(v: PageTableIndex) -> Self {
        let mut ret: Self = v.context_id.try_into().unwrap();

        ret <<= 1;
        ret |= usize::from(u16::from(v.segment));

        ret <<= 6;
        ret |= Self::try_from(v.page_number).unwrap();

        ret
    }
}

impl From<Word> for PageTableIndex {
    /// Converting word written to control register to the page table index,
    /// used when the kernel code will fill the page table.
    fn from(v: Word) -> Self {
        let mut vv = v;

        let page_number: u6 = field!(vv, 6);
        vv >>= 6;

        let segment: VirtualMemorySegment = (vv & 1).try_into().unwrap();
        vv >>= 1;

        let context_id: u6 = field!(vv, 6);

        PageTableIndex{ context_id, segment, page_number }
    }
}

#[derive(Copy, Clone, Debug)]
struct PageTableRecord {
    readable: bool,
    writable: bool,
    frame_number: u14
}

impl From<Word> for PageTableRecord {
    /// Converting word written to control register to the page table record,
    /// used when the kernel code will fill the page table.
    fn from(v: Word) -> PageTableRecord {
        let mut vv = v;

        let frame_number: u14 = field!(vv, 14);
        vv >>= 14;

        let writable = (vv & 1) != 0;
        vv >>= 1;

        let readable = (vv & 1) != 0;

        PageTableRecord{ readable, writable, frame_number }
    }
}

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
    pub fn get_gpr(&self, index: GprIndex) -> Word {
        if index == GprIndex::new(0) {
            0
        } else {
            self.gpr[usize::try_from(index).unwrap() - 1]
        }
    }

    fn set_gpr(&mut self, index: GprIndex, value: Word) {
        if index > GprIndex::new(0) {
            self.gpr[usize::try_from(index).unwrap() - 1] = value
        }
    }

    fn get_cr(&self, index: CrIndex) -> Word {
        todo!();
    }

    fn set_cr(&mut self, index: CrIndex, value: Word) {
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
                PhysicalMemoryAddress::from(page.frame_number) << PAGE_SIZE_BITS |
                PhysicalMemoryAddress::from(page_offset)
            )
        }
    }

    fn write_memory_mapping(&mut self, page_table_index: PageTableIndex, record: PageTableRecord) {
        self.page_table[usize::from(page_table_index)] = record;
    }

    fn step(&mut self, opcode: Word) -> anyhow::Result<()> {
        include!(concat!(env!("OUT_DIR"), "/instruction_handler.rs"));
    }
}

#[derive(Error,Debug)]
pub enum EmulatorError {
    #[error("Attempting to access non-mapped physical memory at {address:#09x} (pc = {pc:#06x})")]
    NonMappedPhysicalMemory { address: PhysicalMemoryAddress, pc: Word }
}

pub trait MemoryMapping: std::fmt::Debug {
    fn read(&self, address: PhysicalMemoryAddress) -> anyhow::Result<Word>;
    fn write(&mut self, address: PhysicalMemoryAddress, value: Word) -> anyhow::Result<()>;
}

fn sign_extend(value: u16, bits: u32) -> u16 {
    assert_eq!(value >> bits, 0);
    let m = 1 << (bits - 1);
    (value ^ m).wrapping_sub(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn sign_extend_examples() {
        assert_eq!(sign_extend(0b10, 2), 0xfffe);
        assert_eq!(sign_extend(0b01, 2), 0b01);
    }
}
