#![feature(adt_const_params)]

pub struct Cpu {
    pub reg: u8,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu { reg: 0 }
    }
}

pub fn load<const IMMEDIATE: bool>(cpu: &mut Cpu, opcode: u8) {
    if IMMEDIATE {
        let value = opcode & 0x0F;
        println!("LOAD imm, {}", value);
        cpu.reg = value;
    } else {
        println!("LOAD zero");
        cpu.reg = 0;
    }
}

pub fn alu<const OP: u8>(cpu: &mut Cpu, _opcode: u8) {
    #[rustfmt::skip]
    let result = match OP {
        0 => { println!("SHL"); cpu.reg << 1 }
        1 => { println!("SHR"); cpu.reg >> 1 }
        2 => { println!("INC"); cpu.reg.wrapping_add(1) }
        3 => { println!("DEC"); cpu.reg.wrapping_sub(1) }
        _ => unreachable!()
    };
    cpu.reg = result;
}

pub const fn bit_to_bool(bit: u8) -> bool {
    bit != 0
}

archibald::instruction_table! {
    type Opcode = u8;

    dispatcher = dispatch;
    context = Cpu;

    "0000'i___" => load<{i}> where {
        i: bool = bit_to_bool(i)
    };

    "0001'00oo" => alu<{o}>;
}

fn main() {
    let mut cpu = Cpu::new();

    println!("--- bool const generic ---");
    dispatch(&mut cpu, 0b0000_0000); // LOAD zero
    dispatch(&mut cpu, 0b0000_1111); // LOAD imm, 15

    println!("--- u8 const generic ---");
    dispatch(&mut cpu, 0b0001_0000); // SHL (op=0)
    dispatch(&mut cpu, 0b0001_0001); // SHR (op=1)
    dispatch(&mut cpu, 0b0001_0010); // INC (op=2)
    dispatch(&mut cpu, 0b0001_0011); // DEC (op=3)

    println!("Final reg value: {}", cpu.reg);
}
