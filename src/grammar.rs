use std::io::{Read, Write};
use std::marker::PhantomData;
use crate::{ReadIntoSelf, ReadSelf, WriteSelf};

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




/// A zero-sized type which consumes a specified number of bytes when reading without allocating any
/// memory.
#[repr(transparent)]
pub struct Padding<const N: usize> {
    _phantom: PhantomData<[u8; N]>,
}

impl<const N: usize> Padding<N> {

    pub fn consume_padding<B: Read>(buffer: &mut B) -> std::io::Result<()> {
        const IO_CHUNK_SIZE: usize = 512;

        if N <= IO_CHUNK_SIZE {
            let mut _dumped = [0u8; N];
            buffer.read_exact(&mut _dumped)?;
        } else {
            let mut _dumped = [0u8; IO_CHUNK_SIZE];
            let mut remaining = N;

            while remaining > 0 {
                let read_size = remaining.min(IO_CHUNK_SIZE);
                buffer.read_exact(&mut _dumped[..read_size])?;
                remaining -= read_size;
            }
        }

        Ok(())
    }

    pub fn apply_padding<B: Write>(buffer: &mut B, padding: u8) -> std::io::Result<()> {
        const IO_CHUNK_SIZE: usize = 512;

        if N <= IO_CHUNK_SIZE {
            let zeros = [padding; N];
            buffer.write_all(&zeros)?;
        } else {
            let zeros = [padding; IO_CHUNK_SIZE];
            let mut remaining = N;

            while remaining > 0 {
                let read_size = remaining.min(IO_CHUNK_SIZE);
                buffer.write_all(&zeros[..read_size])?;
                remaining -= read_size;
            }
        }

        Ok(())
    }
}

impl<const N: usize> ReadSelf for Padding<N> {
    fn read_from<B: Read>(buffer: &mut B) -> std::io::Result<Self> {
        Self::consume_padding(buffer)?;

        Ok(Padding {
            _phantom: PhantomData,
        })
    }
}

impl<const N: usize> ReadIntoSelf for Padding<N> {
    fn read_into<B: Read>(&mut self, buffer: &mut B) -> std::io::Result<()> {
        Self::consume_padding(buffer)
    }
}

impl<const N: usize> WriteSelf for Padding<N> {
    fn write_self<B: Write>(&self, buffer: &mut B) -> std::io::Result<()> {
        Self::apply_padding(buffer, 0)
    }
}

