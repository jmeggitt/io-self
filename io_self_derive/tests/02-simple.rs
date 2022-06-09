#![allow(dead_code)]
use io_self_derive::ReadSelf;

#[derive(ReadSelf)]
pub struct Foo {
    a: u8,
    b: u8,
}

#[derive(ReadSelf)]
pub struct Bar(u8, u8);

fn main() {}
