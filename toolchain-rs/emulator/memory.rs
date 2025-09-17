use ux::*; // Non-standard integer types

use crate::emulator::{cpu::PhysicaMemory, cpu_types::*};
use crate::image::load_ihex;

use std::path::Path;

#[derive(Clone)]
pub struct Ram {
    data: Box<[Word]>,
}

impl Ram {
    /// Construct new zero filled memory
    pub fn new(size: u32) -> Self {
        Ram {
            data: vec![0; size.try_into().unwrap()].into_boxed_slice(),
        }
    }

    /// Construct new memory with random content
    pub fn with_rng(size: u32, rng: &mut impl rand::Rng) -> Self {
        let mut vec = Vec::with_capacity(size.try_into().unwrap());
        vec.extend((0..size).map(|_| rng.gen::<Word>()));
        Ram {
            data: vec.into_boxed_slice(),
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
        Ok(Rom {
            data: load_ihex(path.as_ref())?,
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
