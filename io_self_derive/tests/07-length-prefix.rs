#![allow(dead_code)]
use io_self_derive::{ReadSelf, WriteSelf};

#[derive(ReadSelf, WriteSelf)]
#[io_self(endian = "little", tag = "u8", length_prefix = "u32")]
pub enum Foo {
    #[io_self(tag = "0x01")]
    Bar(u16, i64),
    #[io_self(tag = "0x3F")]
    Baz { a: u8, b: u64, c: i64 },
}

fn main() {}
