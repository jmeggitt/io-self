#![allow(dead_code)]
use io_self_derive::{ReadSelf, WriteSelf};

#[derive(ReadSelf, WriteSelf)]
pub struct Foo {
    a: [u8; 6],
}

#[derive(ReadSelf, WriteSelf)]
pub struct Bar([Foo; 5]);

fn main() {}
