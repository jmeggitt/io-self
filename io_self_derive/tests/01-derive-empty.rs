#![allow(dead_code)]
use io_self_derive::ReadSelf;

#[derive(ReadSelf)]
pub struct Foo;

#[derive(ReadSelf)]
pub struct Bar {}

fn main() {}
