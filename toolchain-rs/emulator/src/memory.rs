use itertools::{chain, repeat_n, Itertools};
use thiserror::Error;
use ux::*; // Non-standard integer types

use crate::cpu::PhysicaMemory;
use crate::cpu_types::*;

use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Ram {
    data: Box<[Word]>,
}

impl Ram {
    pub fn new(size: u32) -> Ram {
        Ram {
            data: vec![0; size.try_into().unwrap()].into_boxed_slice(),
        }
    }
}

impl PhysicaMemory for Ram {
    fn max_address(&self) -> u24 {
        (self.data.len() - 1).try_into().unwrap()
    }
    fn read(&self, address: u24) -> Option<Word> {
        Some(self.data.as_ref()[usize::try_from(address).unwrap()])
    }

    fn write(&mut self, address: u24, value: Word) -> Option<()> {
        self.data.as_mut()[usize::try_from(address).unwrap()] = value;
        Some(())
    }
}

#[derive(Clone)]
pub struct Rom {
    data: Box<[Word]>,
}

impl Rom {
    pub fn from_ihex<P: AsRef<Path>>(path: P) -> anyhow::Result<Rom> {
        let file_str = std::fs::read_to_string(&path)?;

        let u8segments = load_ihex_segments(&file_str, Some(path.as_ref()))?;
        Self::from_segments(&u8segments, Some(path.as_ref()))
    }

    fn from_segments(u8segments: &[U8Segment], file: Option<&Path>) -> anyhow::Result<Rom> {
        if u8segments.is_empty() {
            Err(LoadingRomError::Empty {
                file: file.map(|x| x.into()),
            })?;
        } else if u8segments[0].offset != 0 {
            Err(LoadingRomError::Offset {
                file: file.map(|x| x.into()),
            })?;
        }

        let start_offset_bytes = u8segments[0].offset;
        let end_offset_bytes = u8segments.last().unwrap().end();
        let size_bytes = end_offset_bytes - start_offset_bytes;

        assert_eq!(start_offset_bytes % 2, 0);
        assert_eq!(size_bytes % 2, 0);

        let size = usize::try_from(size_bytes / 2).unwrap();

        let mut data = Vec::with_capacity(size);

        // Fill in values from the first segment:
        data.extend(u8segments[0].iter_u16());

        // Fill in values from the following segments, with the gap filling zeros in between
        data.extend(u8segments.iter().tuple_windows().flat_map(
            |(prev_segment, current_segment)| {
                let gap_bytes =
                    usize::try_from(current_segment.offset - prev_segment.end()).unwrap();
                assert_eq!(gap_bytes % 2, 0);
                chain!(repeat_n(0u16, gap_bytes / 2), current_segment.iter_u16())
            },
        ));

        assert_eq!(data.len(), size);

        Ok(Rom {
            data: data.into_boxed_slice(),
        })
    }
}

impl PhysicaMemory for Rom {
    fn max_address(&self) -> u24 {
        (self.data.len() - 1).try_into().unwrap()
    }

    fn read(&self, address: u24) -> Option<Word> {
        Some(self.data.as_ref()[usize::try_from(address).unwrap()])
    }

    fn write(&mut self, _address: u24, _value: Word) -> Option<()> {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct U8Segment {
    offset: u32,
    data: Vec<u8>,
}

impl U8Segment {
    fn end(&self) -> u32 {
        self.offset + u32::try_from(self.data.len()).unwrap()
    }

    /// Iterates over elements of this segment as big endian u16 (Pickle risc memory layout)
    fn iter_u16<'a>(&'a self) -> impl Iterator<Item = u16> + 'a {
        assert!(self.data.len() % 2 == 0);
        self.data
            .iter()
            .tuples()
            .map(|(high, low)| u16::from_be_bytes([*high, *low]))
    }
}

/// Load segments from the ihex file and sort them.
/// Skips over empty segments.
fn load_ihex_segments(file_str: &str, file: Option<&Path>) -> anyhow::Result<Vec<U8Segment>> {
    let mut ret: Vec<U8Segment> = Vec::new();
    let mut address_base: u32 = 0;
    for record in ihex::Reader::new(&file_str) {
        match record? {
            ihex::Record::Data { offset: _, value } if value.is_empty() => {} // skip over empty records
            ihex::Record::Data { offset, value } => {
                let offset_with_base = address_base + u32::from(offset);
                if offset % 2 != 0 || value.len() % 2 != 0 {
                    Err(LoadingRomError::OddRecord {
                        file: file.map(|x| x.into()),
                        offset: offset_with_base,
                        size: value.len().try_into().unwrap(),
                    })?;
                }
                ret.push(U8Segment {
                    offset: offset_with_base,
                    data: value,
                })
            }
            ihex::Record::ExtendedLinearAddress(ext) => {
                address_base = u32::from(ext) << 16;
            }
            ihex::Record::EndOfFile => break,
            other => Err(LoadingRomError::UnsupportedRecordType {
                file: file.map(|x| x.into()),
                record: format!("{:?}", other),
            })?,
        }
    }

    ret.sort_unstable_by_key(|segment| segment.offset);
    Ok(ret)
}

#[derive(Debug, Error, PartialEq, Eq)]
enum LoadingRomError {
    #[error("Unsupported record type {record} in file {file:?}")]
    UnsupportedRecordType {
        file: Option<PathBuf>,
        record: String,
    },
    #[error("Only even offsets and even record sizes are supported (file {file:?}, {offset:#09x}+{size}B)")]
    OddRecord {
        file: Option<PathBuf>,
        offset: u32,
        size: u32,
    },
    #[error("Segments are overlapping (file {file:?}, {offset:#09x}+{size}B)")]
    Overlapping {
        file: Option<PathBuf>,
        offset: u32,
        size: u32,
    },
    #[error("No data found in {file:?}")]
    Empty { file: Option<PathBuf> },
    #[error("File {file:?} does not start at offset 0")]
    Offset { file: Option<PathBuf> },
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[test]
    fn test_u8segment_end() {
        let segment = U8Segment {
            offset: 123,
            data: vec![0; 1],
        };
        assert_eq!(segment.end(), 124);
    }

    #[test]
    fn test_u8segment_iter_u16() {
        let segment = U8Segment {
            offset: 123,
            data: vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
        };
        assert_eq!(
            segment.iter_u16().collect::<Vec<_>>(),
            vec![0x0102, 0x0304, 0x0506]
        );
    }

    #[test]
    fn test_load_ihex() {
        let ihex = ":040010001122334442";
        let segments = load_ihex_segments(ihex, None).unwrap();
        assert_eq!(
            segments,
            vec![U8Segment {
                offset: 0x0010,
                data: vec![0x11, 0x22, 0x33, 0x44]
            }]
        );
    }

    #[test]
    fn test_load_ihex_extended_address() {
        let ihex = ":040010001122334442\n:02000004FFFFFC\n:040010001122334442";
        let segments = load_ihex_segments(ihex, None).unwrap();
        assert_eq!(
            segments,
            vec![
                U8Segment {
                    offset: 0x00000010,
                    data: vec![0x11, 0x22, 0x33, 0x44]
                },
                U8Segment {
                    offset: 0xffff0010,
                    data: vec![0x11, 0x22, 0x33, 0x44]
                },
            ]
        );
    }

    #[test]
    fn test_load_ihex_unsupported() {
        let ihex = ":020000021200EA";
        let err = load_ihex_segments(ihex, None).unwrap_err();
        assert!(matches!(
            err.downcast_ref::<LoadingRomError>().unwrap(),
            LoadingRomError::UnsupportedRecordType {
                file: None,
                record: _
            }
        ));
    }

    #[test]
    fn test_load_ihex_odd_offset() {
        let ihex = ":040011001122334441";
        let err = load_ihex_segments(ihex, None).unwrap_err();
        assert!(matches!(
            err.downcast_ref::<LoadingRomError>().unwrap(),
            LoadingRomError::OddRecord {
                file: None,
                offset: 0x0011,
                size: 4
            }
        ));
    }

    #[test]
    fn test_load_ihex_odd_length() {
        let ihex = ":05001000112233440041";
        let err = load_ihex_segments(ihex, None).unwrap_err();
        assert!(matches!(
            err.downcast_ref::<LoadingRomError>().unwrap(),
            LoadingRomError::OddRecord {
                file: None,
                offset: 0x0010,
                size: 5
            }
        ));
    }

    #[proptest]
    fn test_rom_from_segments_one(#[strategy(1usize..256usize)] length: usize) {
        let rom = Rom::from_segments(
            &vec![U8Segment {
                offset: 0,
                data: vec![0x00; length * 2],
            }],
            None,
        )
        .unwrap();
        assert_eq!(rom.data.len(), length);
    }

    #[test]
    fn test_rom_from_segments_empty() {
        match Rom::from_segments(&vec![], None) {
            Err(e) => {
                assert!(matches!(
                    e.downcast_ref::<LoadingRomError>().unwrap(),
                    LoadingRomError::Empty { file: None }
                ));
            }
            _ => panic!("Did not result in an error"),
        }
    }

    #[proptest]
    fn test_rom_from_segments_offset(
        #[strategy(1u32..)] offset: u32,
        #[strategy(1usize..256usize)] length: usize,
    ) {
        let rom = Rom::from_segments(
            &vec![U8Segment {
                offset,
                data: vec![0x00; length * 2],
            }],
            None,
        );
        match rom {
            Err(e) => {
                assert!(matches!(
                    e.downcast_ref::<LoadingRomError>().unwrap(),
                    LoadingRomError::Offset { file: None }
                ));
            }
            _ => panic!("Did not result in an error"),
        }
    }

    #[proptest]
    fn test_rom_from_segments_three(
        #[strategy(1usize..256usize)] length1: usize,
        #[strategy(1usize..256usize)] length2: usize,
        #[strategy(1usize..256usize)] gap: usize,
        #[strategy(1usize..256usize)] length3: usize,
    ) {
        let offset2 = u32::try_from(length1).unwrap();
        let offset3 = offset2 + u32::try_from(length2 + gap).unwrap();

        let rom = Rom::from_segments(
            &vec![
                U8Segment {
                    offset: 0,
                    data: vec![0x01; length1 * 2],
                },
                U8Segment {
                    offset: offset2 * 2u32,
                    data: vec![0x02; length2 * 2],
                },
                U8Segment {
                    offset: offset3 * 2u32,
                    data: vec![0x03; length3 * 2],
                },
            ],
            None,
        )
        .unwrap();

        assert_eq!(rom.data.len(), length1 + length2 + gap + length3);

        println!("{:?}", rom.data);

        assert!(rom.data[..length1].iter().all(|x| *x == 0x0101));
        assert!(rom.data[length1..length1 + length2]
            .iter()
            .all(|x| *x == 0x0202));
        assert!(rom.data[length1 + length2..length1 + length2 + gap]
            .iter()
            .all(|x| *x == 0));
        assert!(rom.data[length1 + length2 + gap..]
            .iter()
            .all(|x| *x == 0x0303));
    }
}
