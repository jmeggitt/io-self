//! Helper traits to help with derive macos
use std::io::{self, Read, Write};
use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt};
use crate::{PositionAware, ReadSelf, WriteSelf};

#[doc(hidden)]
pub use byteorder::{LittleEndian, BigEndian};

pub trait ReadSelfEndian<O>: Sized {
    fn read_from<B: Read + PositionAware>(buffer: &mut B) -> io::Result<Self>;
}

impl<O, T: ReadSelf> ReadSelfEndian<O> for T {
    #[inline(always)]
    fn read_from<B: Read + PositionAware>(buffer: &mut B) -> io::Result<Self> {
        <T as ReadSelf>::read_from(buffer)
    }
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




