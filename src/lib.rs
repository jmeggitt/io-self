use std::io;
use std::io::{Read, Write};

pub mod grammar;
mod helper;

pub use helper::*;


pub trait ReadSelf: Sized {
    fn read_from<B: Read>(buffer: &mut B) -> io::Result<Self>;

    fn read_exact_from<F: FromIterator<Self>, B: Read>(buffer: &mut B, count: usize) -> io::Result<F> {
        // TODO: Find better solution. I didn't want to pre-allocate space, but the result can't be
        // unwrapped in the iterator.
        let mut collected = Vec::with_capacity(count);

        for _ in 0..count {
            collected.push(Self::read_from(buffer)?);
        }

        Ok(F::from_iter(collected))
    }
}


/// At times it may be desirable to be able to reuse existing memory without needing to copy
/// `Self`. This variant works similarly to `ReadSelf`, but reads into an existing instance of
/// `Self`.
pub trait ReadIntoSelf {
    fn read_into<B: Read>(&mut self, buffer: &mut B) -> io::Result<()>;
}

impl<T: ReadIntoSelf> ReadIntoSelf for [T] {
    fn read_into<B: Read>(&mut self, buffer: &mut B) -> io::Result<()> {
        self.iter_mut().read_into(buffer)
    }
}

impl<'a, T: 'a + ReadIntoSelf, I: Iterator<Item=&'a mut T>> ReadIntoSelf for I {
    fn read_into<B: Read>(&mut self, buffer: &mut B) -> io::Result<()> {
        for item in self {
            item.read_into(buffer)?;
        }
        Ok(())
    }
}


pub trait WriteSelf: Sized {
    fn write_self<B: Write>(&self, buffer: &mut B) -> io::Result<()>;
}


