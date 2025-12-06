#![feature(adt_const_params)]

use std::marker::ConstParamTy;

pub struct Vm {
    pub r0: u32,
    pub r1: u32,
    pub r2: u32,
    pub r3: u32,
    pub pc: usize,
    pub memory: Vec<u8>,
}

impl Vm {
    pub fn new(program: &[u8]) -> Self {
        Vm {
            r0: 0,
            r1: 0,
            r2: 0,
            r3: 0,
            pc: 0,
            memory: program.to_vec(),
        }
    }

    #[inline(always)]
    fn fetch8(&mut self, addr: usize) -> u8 {
        self.memory[addr]
    }

    #[inline(always)]
    fn get_reg(&self, reg: Register) -> u32 {
        match reg {
            Register::R0 => self.r0,
            Register::R1 => self.r1,
            Register::R2 => self.r2,
            Register::R3 => self.r3,
        }
    }

    #[inline(always)]
    fn set_reg(&mut self, reg: Register, value: u32) {
        match reg {
            Register::R0 => self.r0 = value,
            Register::R1 => self.r1 = value,
            Register::R2 => self.r2 = value,
            Register::R3 => self.r3 = value,
        }
    }
}

#[derive(ConstParamTy, PartialEq, Eq)]
pub enum Register {
    R0 = 0,
    R1 = 1,
    R2 = 2,
    R3 = 3,
}

impl std::fmt::Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Register::R0 => write!(f, "r0"),
            Register::R1 => write!(f, "r1"),
            Register::R2 => write!(f, "r2"),
            Register::R3 => write!(f, "r3"),
        }
    }
}

pub fn impl_add<const REG: Register>(vm: &mut Vm, opcode: u8) {
    println!("add {}, {}", REG, opcode & 0x0F);

    let imm = (opcode & 0x0F) as u32;
    let reg = vm.get_reg(REG);
    vm.set_reg(REG, reg.wrapping_add(imm));
}

pub fn impl_move<const DEST: Register, const SRC: Register>(vm: &mut Vm, _opcode: u8) {
    println!("move {}, {}", DEST, SRC);

    let value = vm.get_reg(SRC);
    vm.set_reg(DEST, value);
}

pub fn impl_load<const REG: Register>(vm: &mut Vm, opcode: u8) {
    println!("load {}, {}", REG, opcode & 0x0F);

    let addr = (opcode & 0x0F) as usize;
    let value = vm.fetch8(addr) as u32;
    vm.set_reg(REG, value);
}

archibald::instruction_table! {
    type Opcode = u8;

    dispatcher = dispatch;
    context = Vm;

    // ADD r0-r3, imm
    "11rr'____" => impl_add<Register::{r}> where {
        r: Register = { 0b00 => R0, 0b01 => R1, 0b10 => R2, 0b11 => R3 }
    };

    // MOVE r0-r3, r0-r3
    "0010'ddss" => impl_move<Register::{d}, Register::{s}> where {
        d: Register = { 0b00 => R0, 0b01 => R1, 0b10 => R2, 0b11 => R3 },
        s: Register = { 0b00 => R0, 0b01 => R1, 0b10 => R2, 0b11 => R3 }
    };

    // LOAD r0-r3, imm
    "01dd'____" => impl_load<Register::{d}> where {
        d: Register = { 0b00 => R0, 0b01 => R1, 0b10 => R2, 0b11 => R3 }
    };
}

fn main() {
    let program = [
        0xCA, // ADD R0, 10
        0x24, // MOVE R1, R0
        0xE4, // ADD R2, 4
        0x2E, // MOVE R3, R2
        0x40, // LOAD R0, 0
    ];

    let mut vm = Vm::new(&program);
    while vm.pc < program.len() {
        let opcode = vm.fetch8(vm.pc);
        dispatch(&mut vm, opcode);
        vm.pc += 1;
    }

    assert_eq!(vm.r1, 10);
    assert_eq!(vm.r2, 4);
    assert_eq!(vm.r3, 4);
    assert_eq!(vm.r0, 0xCA);
}
