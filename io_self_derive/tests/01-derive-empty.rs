#![allow(dead_code)]
use io_self_derive::{ReadSelf, WriteSelf};

#[derive(ReadSelf, WriteSelf)]
pub struct Foo;

#[derive(ReadSelf, WriteSelf)]
pub struct Bar {}

fn main() {}
