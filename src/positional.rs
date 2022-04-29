use std::io::{self, Read, Seek, Write};

/// This trait allows access to the current position of a stream. Essentially, it limits the
/// functionality of `std::io::Seek` to `stream_position`. This trait is helpful for cases where
/// knowing the offset from the start of a stream is important, but the cursor will never be moved.
pub trait PositionAware {
    fn position(&mut self) -> io::Result<u64>;
}

impl<S: Seek> PositionAware for S {
    fn position(&mut self) -> io::Result<u64> {
        self.stream_position()
    }
}

/// A minimal position aware reader.
pub struct ReadCounter<R> {
    reader: R,
    position: u64,
}

impl<R: Read> ReadCounter<R> {
    pub fn new(reader: R) -> Self {
        ReadCounter {
            reader,
            position: 0,
        }
    }
}

impl<R: Read> Read for ReadCounter<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let res = self.reader.read(buf)?;
        self.position += res as u64;
        Ok(res)
    }
}

impl<R> PositionAware for ReadCounter<R> {
    fn position(&mut self) -> io::Result<u64> {
        Ok(self.position)
    }
}

/// A minimal position aware writer.
pub struct WriteCounter<W> {
    writer: W,
    position: u64,
}

impl<W: Write> WriteCounter<W> {
    pub fn new(writer: W) -> Self {
        WriteCounter {
            writer,
            position: 0,
        }
    }
}

impl<W: Write> Write for WriteCounter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let res = self.writer.write(buf)?;
        self.position += res as u64;
        Ok(res)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl<W> PositionAware for WriteCounter<W> {
    fn position(&mut self) -> io::Result<u64> {
        Ok(self.position)
    }
}
