#![allow(dead_code)]
use io_self_derive::{ReadSelf, WriteSelf};

#[derive(ReadSelf, WriteSelf)]
#[io_self(endian = "big")]
pub struct Foo {
    a: u32,
    #[io_self(length_prefix = "u16")]
    b: Vec<u8>,
    c: i64,
}

fn main() {}
