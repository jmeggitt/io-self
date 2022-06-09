#![allow(dead_code)]
use io_self_derive::ReadSelf;

#[derive(ReadSelf)]
pub struct Foo {
    a: [u8; 6],
}

#[derive(ReadSelf)]
pub struct Bar ([Foo; 5]);

fn main() {}
