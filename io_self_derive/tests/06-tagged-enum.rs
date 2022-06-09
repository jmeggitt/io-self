#![allow(dead_code)]
use io_self_derive::ReadSelf;

#[derive(ReadSelf)]
#[io_self(endian="big", tagged="u8")]
pub enum Foo {
    #[io_self(tag="0x01")]
    Bar (u16, i64),
    #[io_self(tag="0x3F")]
    Baz {
        a: u8,
        b: u64,
        c: i64,
    },
}

fn main() {}
