#![allow(dead_code)]
use io_self_derive::{ReadSelf, WriteSelf};

#[derive(ReadSelf, WriteSelf)]
#[io_self(endian = "big")]
pub struct Foo {
    a: [u64; 6],
}

fn main() {}
