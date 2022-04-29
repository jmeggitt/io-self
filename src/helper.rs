use smallvec::{Array, SmallVec};
use std::io::{self, Read, Write};
use std::marker::PhantomData;

/// When creating a `std::io::Read` wrapper that performs any form of processing, you can quickly
/// run into issues when a buffer does not have enough space for the what you want to write to it.
/// This type works as an intermediate buffer which keeps track of bytes which could not be fit into
/// the previous buffer and adds them to the next call. Because it can buffer additional data, it
/// also enables the use of `write!` to make data handling a bit easier on the user.
///
/// Additionally, `OverflowBuffer` is implemented using `SmallVec` so frequent small overflows will
/// not result in allocation to the heap.
pub struct OverflowBuffer<const N: usize = 8>
where
    [u8; N]: Array,
{
    buffer: SmallVec<[u8; N]>,
    index: usize,
}

impl<const N: usize> Default for OverflowBuffer<N>
where
    [u8; N]: Array<Item = u8>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> OverflowBuffer<N>
where
    [u8; N]: Array<Item = u8>,
{
    pub fn new() -> Self {
        OverflowBuffer {
            buffer: SmallVec::default(),
            index: 0,
        }
    }

    fn take_overflow(&mut self, buffer: &mut [u8]) -> usize {
        // Copy overflow bytes and adjust index
        let copy_len = (self.buffer.len() - self.index).min(buffer.len());
        buffer[..copy_len].copy_from_slice(&self.buffer[self.index..self.index + copy_len]);
        self.index += copy_len;

        // If we consumed the entire buffer, reset it so we can re-use the memory
        if self.index >= self.buffer.len() {
            self.buffer.clear();
            self.index = 0;
        }

        copy_len
    }

    pub fn for_buffer<'a: 'b, 'b>(
        &'a mut self,
        buffer: &'b mut [u8],
    ) -> Result<OverflowingWriter<'a, 'b, N>, usize> {
        // Copy remaining overflow into buffer
        let write_len = self.take_overflow(buffer);

        if write_len == buffer.len() {
            return Err(write_len);
        }

        Ok(OverflowingWriter {
            buffer,
            index: write_len,
            overflow: self,
        })
    }
}

impl<const N: usize> Read for OverflowBuffer<N>
where
    [u8; N]: Array<Item = u8>,
{
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        Ok(self.take_overflow(buffer))
    }
}

impl<const N: usize> Write for OverflowBuffer<N>
where
    [u8; N]: Array<Item = u8>,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub struct OverflowingWriter<'a, 'b, const N: usize = 8>
where
    [u8; N]: Array<Item = u8>,
{
    buffer: &'b mut [u8],
    index: usize,
    overflow: &'a mut OverflowBuffer<N>,
}

impl<'a, 'b, const N: usize> OverflowingWriter<'a, 'b, N>
where
    [u8; N]: Array<Item = u8>,
{
    pub fn ok(self) -> io::Result<usize> {
        Ok(self.index)
    }

    pub fn has_space(&self) -> bool {
        self.index < self.buffer.len()
    }
}

impl<'a, 'b, const N: usize> Write for OverflowingWriter<'a, 'b, N>
where
    [u8; N]: Array<Item = u8>,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Copy as much of buf to output buffer as possible
        let copy_len = (self.buffer.len() - self.index).min(buf.len());
        self.buffer[self.index..self.index + copy_len].copy_from_slice(&buf[..copy_len]);
        self.index += copy_len;

        // If there is any remaining data, put it into the overflow
        if copy_len < buf.len() {
            self.overflow.buffer.extend_from_slice(&buf[copy_len..]);
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub trait AbortingFromIterator<T, E>: Iterator<Item = Result<T, E>> {
    fn aborting_from_iter<F: FromIterator<T>>(self) -> Result<F, E>;
}

impl<I, T, E> AbortingFromIterator<T, E> for I
where
    I: Iterator<Item = Result<T, E>>,
{
    fn aborting_from_iter<F: FromIterator<T>>(self) -> Result<F, E> {
        let mut unwrapper = AbortingIter {
            iter: self,
            err: None,
            _phantom: PhantomData,
        };

        let res = F::from_iter(&mut unwrapper);

        match unwrapper.err {
            Some(e) => Err(e),
            None => Ok(res),
        }
    }
}

struct AbortingIter<I, T, E> {
    iter: I,
    err: Option<E>,
    _phantom: PhantomData<T>,
}

impl<I, T, E> Iterator for AbortingIter<I, T, E>
where
    I: Iterator<Item = Result<T, E>>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.err.is_some() {
            return None;
        }

        match self.iter.next()? {
            Ok(v) => Some(v),
            Err(e) => {
                self.err = Some(e);
                None
            }
        }
    }
}
