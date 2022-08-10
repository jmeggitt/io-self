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

#[derive(ReadSelf, WriteSelf)]
#[io_self(endian = "big", tag = "u8")]
pub enum Bar {
    #[io_self(tag="0x23")]
    Fizz {
        a: u32,
        #[io_self(length_prefix = "u16")]
        b: Vec<u64>,
    },
    #[io_self(tag="0x89")]
    Baz (#[io_self(length_prefix = "u16")] Vec<u32>),
}

fn main() {}
