#![allow(dead_code)]
use io_self_derive::ReadSelf;

#[derive(ReadSelf)]
#[io_self(endian="little")]
pub struct Foo {
    a: u8,
    b: u16,
    c: u32,
    d: u64,
    e: u128,
    f: i8,
    g: i16,
    h: i32,
    i: i64,
    j: i128,
}

#[derive(ReadSelf)]
#[io_self(endian="big")]
pub struct Bar (u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

fn main() {}
