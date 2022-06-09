#![allow(dead_code)]
use io_self_derive::{ReadSelf, WriteSelf};

#[derive(ReadSelf, WriteSelf)]
pub struct Foo {
    a: u8,
    b: u8,
}

#[derive(ReadSelf, WriteSelf)]
pub struct Bar(u8, u8);

fn main() {}
