# Archibald

A Rust proc-macro for building high-performance instruction decoders with **compile-time branch elimination** using const generics.

## What?
Since this allows you to declaratively map specific bit patterns (e.g. `1010xxyy`) to constant generic parameters the compiler will treat each specialization as a new "function", effectively eliminating all branches that operate on the parameters. See the following example.

## Examples
- [Brainfuck](./examples/brainfuck.rs) - Simple Brainfuck interpreter without const generics
- [Simple VM](./examples/simple_vm.rs) - Manual enum mapping with const generics
- [Const Functions](./examples/const_fn.rs) - Using const functions to map bit patterns
- [Primitives](./examples/primitive.rs) - Using primitive types (bool, u8) as const generics

Complete example showing all features:

```rust
#![feature(adt_const_params)]

#[derive(ConstParamTy, PartialEq, Eq)]
pub enum Register { R0, R1, R2, R3 }

pub const fn decode_register(bits: u8) -> Register {
    match bits {
        0b00 => Register::R0,
        0b01 => Register::R1,
        0b10 => Register::R2,
        0b11 => Register::R3,
        _ => unreachable!()
    }
}

pub const fn bit_to_bool(bit: u8) -> bool { bit != 0 }

archibald::instruction_table! {
    type Opcode = u8;
    dispatcher = dispatch;
    context = Cpu;

    // 1. Manual mapping with typed variable
    "0000'00rr" => add<{r}> where {
        r: Register = { 0b00 => R0, 0b01 => R1, 0b10 => R2, 0b11 => R3 }
    };

    // 2. Const function with typed variable
    "0001'00rr" => sub<{r}> where {
        r: Register = decode_register(r)
    };

    // 3. Primitive bool with const function
    "0011'000c" => shift<{c}> where {
        c: bool = bit_to_bool(c)
    };

    // 4. Primitive u8 - no where clause needed
    "0100'00oo" => alu<{o}>;

    // 5. Multiple variables (typed)
    "0101'ddss" => mov<{d}, {s}> where {
        d: Register = decode_register(d),
        s: Register = decode_register(s)
    };

    // 6. Mixed: u8 + bool
    "0111'ooc_" => compare<{o}, {c}> where {
        c: bool = bit_to_bool(c)
        // o uses raw u8 automatically
    };

    // 7. Wildcards for immediate values
    "1000'rr__" => load_imm<{r}> where {
        r: Register = decode_register(r)
        // Bottom 2 bits are wildcards, extracted via opcode parameter
    };

    // 8. Fixed opcode (no variables)
    "1111'1111" => halt;
}

// Arbitrary handler examples
pub fn add<const R: Register>(cpu: &mut Cpu, opcode: u8) { /* ... */ }
pub fn alu<const OP: u8>(cpu: &mut Cpu, opcode: u8) { /* ... */ }
pub fn compare<const OP: u8, const CARRY: bool>(cpu: &mut Cpu, opcode: u8) { /* ... */ }
pub fn halt(cpu: &mut Cpu, opcode: u8) { /* ... */ }
```

The resulting `dispatch` function looks like this (taken from `const_fn.rs` using `cargo expand`):

```rs
#[inline]
pub fn dispatch(ctx: &mut (), opcode: u8) {
    match opcode {
        op if op & 240u8 == 0u8 => handler::<{ decode_mode(0u8) }>(ctx, opcode),
        op if op & 240u8 == 16u8 => handler::<{ decode_mode(1u8) }>(ctx, opcode),
        op if op & 240u8 == 32u8 => handler::<{ decode_mode(2u8) }>(ctx, opcode),
        op if op & 240u8 == 48u8 => handler::<{ decode_mode(3u8) }>(ctx, opcode),
        op if op & 240u8 == 64u8 => handler::<{ Mode::A }>(ctx, opcode),
        op if op & 240u8 == 80u8 => handler::<{ Mode::B }>(ctx, opcode),
        op if op & 240u8 == 96u8 => handler::<{ Mode::C }>(ctx, opcode),
        op if op & 240u8 == 112u8 => handler::<{ Mode::D }>(ctx, opcode),
        _ => {
            ::core::panicking::panic_fmt(
                format_args!("Unhandled opcode: 0x{0:02X}", opcode),
            );
        }
    }
}
```

## Syntax
- `0` / `1` - Fixed bits that must match exactly
- `_` - Wildcard bits (don't care, accessible via opcode parameter)
- `a-z` - Variable bits (extracted and bound to const generics)
- `'` - Visual separator (ignored, for readability)
- Supports `u8`, `u16`, `u32`, `u64` opcode sizes

The compiler evaluates all const expressions at compile-time, enabling full branch elimination!

## Pattern Priority
**Important:** When patterns overlap, more specific patterns must come **first**!

```rust
// CORRECT
"0101'____" => handler_specific;  // Matches 0x50-0x5F
"01__'____" => handler_generic;   // Matches 0x40-0x7F

// WRONG
"01__'____" => handler_generic;   // Matches 0x40-0x7F
"0101'____" => handler_specific;  // Never reached
```

## Installation
Add to your `Cargo.toml`:

```toml
[dependencies]
archibald = { git = "https://github.com/ioncodes/archibald" }
```

Enable the required nightly feature in your crate if you're intending to use enums in your const generic parameters:

```rust
#![feature(adt_const_params)]
```