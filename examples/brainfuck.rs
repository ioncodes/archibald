pub struct Brainfuck {
    pub memory: [u8; 30000],
    pub ptr: usize,
    pub pc: usize,
    pub program: Vec<u8>,
}

impl Brainfuck {
    pub fn new(program: &[u8]) -> Self {
        Brainfuck {
            memory: [0; 30000],
            ptr: 0,
            pc: 0,
            program: program.to_vec(),
        }
    }
}

// Instruction implementations
pub fn op_inc_ptr(vm: &mut Brainfuck, _opcode: u8) {
    vm.ptr = (vm.ptr + 1) % 30000;
}

pub fn op_dec_ptr(vm: &mut Brainfuck, _opcode: u8) {
    vm.ptr = if vm.ptr == 0 { 29999 } else { vm.ptr - 1 };
}

pub fn op_inc_val(vm: &mut Brainfuck, _opcode: u8) {
    vm.memory[vm.ptr] = vm.memory[vm.ptr].wrapping_add(1);
}

pub fn op_dec_val(vm: &mut Brainfuck, _opcode: u8) {
    vm.memory[vm.ptr] = vm.memory[vm.ptr].wrapping_sub(1);
}

pub fn op_output(vm: &mut Brainfuck, _opcode: u8) {
    print!("{}", vm.memory[vm.ptr] as char);
}

pub fn op_input(vm: &mut Brainfuck, _opcode: u8) {
    use std::io::Read;
    let mut buffer = [0u8; 1];
    std::io::stdin().read_exact(&mut buffer).unwrap();
    vm.memory[vm.ptr] = buffer[0];
}

pub fn op_loop_start(vm: &mut Brainfuck, _opcode: u8) {
    if vm.memory[vm.ptr] == 0 {
        // Jump forward to matching ]
        let mut depth = 1;
        while depth > 0 {
            vm.pc += 1;
            match vm.program[vm.pc] {
                b'[' => depth += 1,
                b']' => depth -= 1,
                _ => {}
            }
        }
    }
}

pub fn op_loop_end(vm: &mut Brainfuck, _opcode: u8) {
    if vm.memory[vm.ptr] != 0 {
        // Jump backward to matching [
        let mut depth = 1;
        while depth > 0 {
            vm.pc -= 1;
            match vm.program[vm.pc] {
                b']' => depth += 1,
                b'[' => depth -= 1,
                _ => {}
            }
        }
    }
}

// Instruction table
archibald::instruction_table! {
    type Opcode = u8;

    dispatcher = dispatch;
    context = Brainfuck;

    // Brainfuck commands map to their ASCII values
    "00111110" => op_inc_ptr;       // '>' (0x3E = 62 = 0b00111110)
    "00111100" => op_dec_ptr;       // '<' (0x3C = 60 = 0b00111100)
    "00101011" => op_inc_val;       // '+' (0x2B = 43 = 0b00101011)
    "00101101" => op_dec_val;       // '-' (0x2D = 45 = 0b00101101)
    "00101110" => op_output;        // '.' (0x2E = 46 = 0b00101110)
    "00101100" => op_input;         // ',' (0x2C = 44 = 0b00101100)
    "01011011" => op_loop_start;    // '[' (0x5B = 91 = 0b01011011)
    "01011101" => op_loop_end;      // ']' (0x5D = 93 = 0b01011101)
}

fn main() {
    let program = b"++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.";

    let mut vm = Brainfuck::new(program);

    while vm.pc < vm.program.len() {
        let opcode = vm.program[vm.pc];

        // Only dispatch valid brainfuck commands, skip others
        if matches!(
            opcode,
            b'>' | b'<' | b'+' | b'-' | b'.' | b',' | b'[' | b']'
        ) {
            dispatch(&mut vm, opcode);
        }

        vm.pc += 1;
    }
}
