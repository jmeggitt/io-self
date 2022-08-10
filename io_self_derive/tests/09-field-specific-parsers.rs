#![allow(dead_code)]

use std::io;
use std::io::{Read, Write};
use io_self_derive::{ReadSelf, WriteSelf};

#[derive(ReadSelf, WriteSelf)]
#[io_self(endian = "big")]
pub struct Foo {
    a: u32,
    #[io_self(read_fn = "read_u64", write_fn = "write_u64")]
    c: *mut u8,
}

fn read_u64<B: Read>(buffer: &mut B) -> io::Result<*mut u8> {
    let mut tmp_buffer = [0u8; 8];
    buffer.read_exact(&mut tmp_buffer)?;

    Ok(u64::from_be_bytes(tmp_buffer) as usize as *mut u8)
}

fn write_u64<B: Write>(val: &*mut u8, buffer: &mut B) -> io::Result<()> {
    buffer.write_all(&(*val as usize as u64).to_be_bytes())
}


fn main() {}
