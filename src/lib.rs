use std::io;
use std::io::{Cursor, Read, Write};

pub mod grammar;
pub mod helper;
pub mod positional;

use helper::AbortingFromIterator;
pub use positional::PositionAware;

pub trait ReadSelf: Sized {
    fn read_from<B: Read + PositionAware>(buffer: &mut B) -> io::Result<Self>;

    fn from_slice(bytes: &[u8]) -> io::Result<Self> {
        let mut buffer = Cursor::new(bytes);
        Self::read_from(&mut buffer)
    }

    fn read_exact_from<F: FromIterator<Self>, B: Read + PositionAware>(
        buffer: &mut B,
        count: usize,
    ) -> io::Result<F> {
        (0..count)
            .into_iter()
            .map(|_| Self::read_from(buffer))
            .aborting_from_iter()
    }
}

/// At times it may be desirable to be able to reuse existing memory without needing to copy
/// `Self`. This variant works similarly to `ReadSelf`, but reads into an existing instance of
/// `Self`.
pub trait ReadIntoSelf {
    fn read_into<B: Read + PositionAware>(&mut self, buffer: &mut B) -> io::Result<()>;
}

impl<T: ReadIntoSelf> ReadIntoSelf for [T] {
    fn read_into<B: Read + PositionAware>(&mut self, buffer: &mut B) -> io::Result<()> {
        self.iter_mut().read_into(buffer)
    }
}

impl<'a, T: 'a + ReadIntoSelf, I: Iterator<Item = &'a mut T>> ReadIntoSelf for I {
    fn read_into<B: Read + PositionAware>(&mut self, buffer: &mut B) -> io::Result<()> {
        for item in self {
            item.read_into(buffer)?;
        }
        Ok(())
    }
}

pub trait WriteSelf: Sized {
    fn write_self<B: Write + PositionAware>(&self, buffer: &mut B) -> io::Result<()>;
}
