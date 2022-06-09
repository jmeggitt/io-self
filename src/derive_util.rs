//! Helper traits to help with derive macos
use crate::{AbortingFromIterator, PositionAware, ReadSelf, WriteSelf};
use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt};
use std::io::{self, Error, ErrorKind, Read, Write};

#[doc(hidden)]
pub use byteorder::{BigEndian, LittleEndian};

pub trait ReadSelfEndian<O>: Sized {
    fn read_from<B: Read + PositionAware>(buffer: &mut B) -> io::Result<Self>;
}

impl<O, T: ReadSelf> ReadSelfEndian<O> for T {
    #[inline(always)]
    fn read_from<B: Read + PositionAware>(buffer: &mut B) -> io::Result<Self> {
        <T as ReadSelf>::read_from(buffer)
    }
}

/// Utility function to help allow the compiler to infer types.
/// TODO: Should this instead be inlined by the derive macro?
#[inline(always)]
pub fn read_with_length<B, T, A, F>(buffer: &mut B, len: usize, parser: F) -> io::Result<A>
where
    B: Read + PositionAware,
    A: FromIterator<T>,
    F: Fn(&mut B) -> io::Result<T>,
{
    (0..len)
        .into_iter()
        .map(|_| parser(buffer))
        .aborting_from_iter()
}

/// Utility function to help allow the compiler to infer types.
/// TODO: Should this instead be inlined by the derive macro?
#[inline(always)]
pub fn write_with_prefix<P, A, B, T, F, G>(
    items: &A,
    buffer: &mut B,
    item_writer: F,
    prefix_writer: G,
) -> io::Result<()>
where
    P: TryFrom<usize>,
    <P as TryFrom<usize>>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    B: Write + PositionAware,
    for<'a> &'a A: IntoIterator<Item = &'a T>,
    F: Fn(&T, &mut B) -> io::Result<()>,
    G: FnOnce(&P, &mut B) -> io::Result<()>,
{
    let iter = items.into_iter();

    // Use size_hint if it returns an exact result, otherwise perform the expensive task of counting
    // the elements in the iterator.
    let count = match iter.size_hint() {
        (x, Some(y)) if x > 0 && x == y => x,
        _ => items.into_iter().count(),
    };

    let length_prefix = match P::try_from(count) {
        Ok(v) => v,
        Err(e) => return Err(Error::new(ErrorKind::Other, e)),
    };
    prefix_writer(&length_prefix, buffer)?;

    let mut found_elements = 0;
    for item in iter {
        found_elements += 1;
        item_writer(item, buffer)?;
    }

    assert_eq!(
        count, found_elements,
        "Iterator size hint lead to incorrect length emitted"
    );
    Ok(())
}

pub trait WriteSelfEndian<O>: Sized {
    fn write_to<B: Write + PositionAware>(&self, buffer: &mut B) -> io::Result<()>;
}

impl<O, T: WriteSelf> WriteSelfEndian<O> for T {
    #[inline(always)]
    fn write_to<B: Write + PositionAware>(&self, buffer: &mut B) -> io::Result<()> {
        <T as WriteSelf>::write_to(self, buffer)
    }
}

macro_rules! impl_for {
    ($name:ty: $read:ident, $write:ident) => {
        impl<O: ByteOrder> ReadSelfEndian<O> for $name {
            #[inline(always)]
            fn read_from<B: Read + PositionAware>(buffer: &mut B) -> io::Result<Self> {
                buffer.$read::<O>()
            }
        }

        impl<O: ByteOrder> WriteSelfEndian<O> for $name {
            #[inline(always)]
            fn write_to<B: Write + PositionAware>(&self, buffer: &mut B) -> io::Result<()> {
                buffer.$write::<O>(*self)
            }
        }
    };
    ($($name:ty: $read:ident, $write:ident);+) => {
        $(impl_for!{$name: $read, $write})+
    }
}

impl_for! {
    u16: read_u16, write_u16;
    u32: read_u32, write_u32;
    u64: read_u64, write_u64;
    u128: read_u128, write_u128;
    i16: read_i16, write_i16;
    i32: read_i32, write_i32;
    i64: read_i64, write_i64;
    i128: read_i128, write_i128
}
