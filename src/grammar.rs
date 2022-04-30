use crate::{PositionAware, ReadIntoSelf, ReadSelf, WriteSelf};
use std::io;
use std::io::{Read, Write};

#[macro_export]
macro_rules! grammar {
    ($($(#[$($macros:tt)+])* $pub:vis struct $name:ident { $($(#[$($field_macros:tt)+])* $field_vis:vis $field:ident: $type:ty),* $(,)? })+) => {
        $(simple_grammar!{
            @impl $(#[$($macros)+])*
            $pub struct $name {
                $($(#[$($field_macros)+])*
                $field_vis $field: $type),*
            }
        })+
    };
    (@impl $(#[$($macros:tt)+])* $pub:vis struct $name:ident { $($(#[$($field_macros:tt)+])* $field_vis:vis $field:ident: $type:ty),* $(,)? }) => {
        $(#[$($macros)+])*
        $pub struct $name {
                $($(#[$($field_macros)+])*
                $field_vis $field: $type),*
        }

        impl Readable for $name {
            fn read<T: Read>(buffer: &mut T) -> io::Result<Self> {
                Ok($name { $($field: <$type as Readable>::read(buffer)?),+ })
            }
        }
    };
}

const IO_CHUNK_SIZE: usize = 512;

pub fn consume_bytes<B: Read>(buffer: &mut B, mut len: usize) -> std::io::Result<()> {
    let mut _dumped = [0u8; IO_CHUNK_SIZE];

    while len > 0 {
        let read_size = len.min(IO_CHUNK_SIZE);
        buffer.read_exact(&mut _dumped[..read_size])?;
        len -= read_size;
    }

    Ok(())
}

pub fn write_padding<B: Write>(buffer: &mut B, byte: u8, mut len: usize) -> std::io::Result<()> {
    let padding = [byte; IO_CHUNK_SIZE];

    while len > 0 {
        let write_size = len.min(IO_CHUNK_SIZE);
        buffer.write_all(&padding[..write_size])?;
        len -= write_size;
    }

    Ok(())
}

/// A zero-sized type which consumes a specified number of bytes when reading without allocating any
/// memory.
#[derive(Default, Debug, Copy, Clone)]
pub struct Padding<const N: usize>;

impl<const N: usize> ReadSelf for Padding<N> {
    fn read_from<B: Read>(buffer: &mut B) -> io::Result<Self> {
        // Self::consume_padding(buffer)?;
        consume_bytes(buffer, N)?;
        Ok(Padding)
    }
}

impl<const N: usize> ReadIntoSelf for Padding<N> {
    fn read_into<B: Read>(&mut self, buffer: &mut B) -> io::Result<()> {
        // Self::consume_padding(buffer)
        consume_bytes(buffer, N)
    }
}

impl<const N: usize> WriteSelf for Padding<N> {
    fn write_self<B: Write>(&self, buffer: &mut B) -> io::Result<()> {
        write_padding(buffer, 0, N)
        // Self::apply_padding(buffer, 0)
    }
}

/// Some file formats require that values be stored at a specific alignment so it can be used
/// directly after being read into memory. From the cases I have seen, these requirements become
/// outdated as newer systems make the performance gains negligible. However it is not unusual to
/// leave the requirement in place for backwards compatability. Position is determined via
/// `PositionAware`, so a user can choose to reset the position for a specific struct if needed.
///
/// Unlike typical alignment, this will align to any arbitrary `N` including zero and non-powers of
/// 2. An alignment of 0 is ignored and padding is defined as adding the smallest padding `p` such
/// that `(position + p) % alignment == 0`.
///
/// When reading, padding dropped without checking if the given values match the padding byte.
#[derive(Default, Debug, Copy, Clone)]
pub struct PadToAlign<const N: u64, const PADDING_BYTE: u8 = 0>;

impl<const N: u64, const P: u8> PadToAlign<N, P> {
    fn padding_for(position: u64) -> u64 {
        if N < 2 {
            return 0;
        }

        let offset = position % N;
        if offset == 0 {
            0
        } else {
            N - offset
        }
    }
}

impl<const N: u64, const P: u8> ReadSelf for PadToAlign<N, P> {
    fn read_from<B: Read + PositionAware>(buffer: &mut B) -> io::Result<Self> {
        let padding = Self::padding_for(buffer.position()?) as usize;
        consume_bytes(buffer, padding)?;
        Ok(PadToAlign)
    }
}

impl<const N: u64, const P: u8> ReadIntoSelf for PadToAlign<N, P> {
    fn read_into<B: Read + PositionAware>(&mut self, buffer: &mut B) -> io::Result<()> {
        let padding = Self::padding_for(buffer.position()?) as usize;
        consume_bytes(buffer, padding)
    }
}

impl<const N: u64, const P: u8> WriteSelf for PadToAlign<N, P> {
    fn write_self<B: Write + PositionAware>(&self, buffer: &mut B) -> io::Result<()> {
        let padding = Self::padding_for(buffer.position()?) as usize;
        write_padding(buffer, P, padding)
    }
}
