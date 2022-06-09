#![allow(dead_code)]
use io_self_derive::ReadSelf;

#[derive(ReadSelf)]
#[io_self(endian="big")]
pub struct Foo {
    a: [u64; 6],
}

fn main() {}
