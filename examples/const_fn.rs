#![feature(adt_const_params)]

use std::marker::ConstParamTy;

#[derive(ConstParamTy, PartialEq, Eq, Debug)]
pub enum Mode {
    A = 0,
    B = 1,
    C = 2,
    D = 3,
}

pub const fn decode_mode(bits: u8) -> Mode {
    match bits {
        0b00 => Mode::A,
        0b01 => Mode::B,
        0b10 => Mode::C,
        0b11 => Mode::D,
        _ => unreachable!(),
    }
}

pub fn handler<const M: Mode>(_ctx: &mut (), opcode: u8) {
    println!("Handler for mode {:?}, opcode: 0x{:02X}", M, opcode);
}

archibald::instruction_table! {
    type Opcode = u8;

    dispatcher = dispatch;
    context = ();

    "00mm'____" => handler<{m}> where {
        m: Mode = decode_mode(m)
    };

    "01mm'____" => handler<{m}> where {
        m: Mode = { 0b00 => A, 0b01 => B, 0b10 => C, 0b11 => D }
    };
}

fn main() {
    let mut ctx = ();

    println!("Testing const function syntax:");
    dispatch(&mut ctx, 0b0000_0000); // Mode::A
    dispatch(&mut ctx, 0b0001_0000); // Mode::B
    dispatch(&mut ctx, 0b0010_0000); // Mode::C
    dispatch(&mut ctx, 0b0011_0000); // Mode::D

    println!("Testing manual mapping syntax:");
    dispatch(&mut ctx, 0b0100_0000); // Mode::A
    dispatch(&mut ctx, 0b0101_0000); // Mode::B
    dispatch(&mut ctx, 0b0110_0000); // Mode::C
    dispatch(&mut ctx, 0b0111_0000); // Mode::D
}
