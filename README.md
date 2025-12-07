# Archibald

A Rust proc-macro for building high-performance instruction decoders with **compile-time branch elimination** using const generics.

## What?
Since this allows you to declaratively map specific bit patterns (e.g. `1010xxyy`) to constant generic parameters the compiler will treat each specialization as a new "function", effectively eliminating all branches that operate on the parameters. See the following example.

## Examples
- [Brainfuck](./examples/brainfuck.rs) - Very simple Brainfuck interpreter without const generics
- [Simple VM](./examples/simple_vm.rs) - Showcases how const generics are used

Using const generics to specialize handlers at compile time:

```rust
archibald::instruction_table! {
    type Opcode = u8;

    dispatcher = dispatch; // "dispatch" is the generated function that dispatches the instruction
    context = Cpu;         // Your abstraction over the emulated context

    // Pattern with variables, expands to 4 specialized opcodes/implementations
    "0001'rr__" => op_inc<Register::{r}> where {
        r: Register = {
            0b00 => R0,
            0b01 => R1,
            0b10 => R2,
            0b11 => R3
        }
    };
}

// The "mastch" is compiled away and instead the binary will contain a "op_inc" specialization for each case
pub fn op_inc<const REG: Register>(vm: &mut Brainfuck, _opcode: u8) {
    match REG {
        Register::R0 => /* ... */,
        Register::R1 => /* ... */,
    }
}
```

The resulting `dispatch` function looks like this (taken from `simple_vm.rs` using `cargo expand`):

```rs
#[inline]
pub fn dispatch(ctx: &mut Vm, opcode: u8) {
    match opcode {
        op if op & 240u8 == 192u8 => impl_add::<{ Register::R0 }>(ctx, opcode),
        op if op & 240u8 == 208u8 => impl_add::<{ Register::R1 }>(ctx, opcode),
        op if op & 240u8 == 224u8 => impl_add::<{ Register::R2 }>(ctx, opcode),
        op if op & 240u8 == 240u8 => impl_add::<{ Register::R3 }>(ctx, opcode),
        32u8 => impl_move::<{ Register::R0 }, { Register::R0 }>(ctx, opcode),
        33u8 => impl_move::<{ Register::R0 }, { Register::R1 }>(ctx, opcode),
        34u8 => impl_move::<{ Register::R0 }, { Register::R2 }>(ctx, opcode),
        35u8 => impl_move::<{ Register::R0 }, { Register::R3 }>(ctx, opcode),
        36u8 => impl_move::<{ Register::R1 }, { Register::R0 }>(ctx, opcode),
        37u8 => impl_move::<{ Register::R1 }, { Register::R1 }>(ctx, opcode),
        38u8 => impl_move::<{ Register::R1 }, { Register::R2 }>(ctx, opcode),
        39u8 => impl_move::<{ Register::R1 }, { Register::R3 }>(ctx, opcode),
        40u8 => impl_move::<{ Register::R2 }, { Register::R0 }>(ctx, opcode),
        41u8 => impl_move::<{ Register::R2 }, { Register::R1 }>(ctx, opcode),
        42u8 => impl_move::<{ Register::R2 }, { Register::R2 }>(ctx, opcode),
        43u8 => impl_move::<{ Register::R2 }, { Register::R3 }>(ctx, opcode),
        44u8 => impl_move::<{ Register::R3 }, { Register::R0 }>(ctx, opcode),
        45u8 => impl_move::<{ Register::R3 }, { Register::R1 }>(ctx, opcode),
        46u8 => impl_move::<{ Register::R3 }, { Register::R2 }>(ctx, opcode),
        47u8 => impl_move::<{ Register::R3 }, { Register::R3 }>(ctx, opcode),
        op if op & 240u8 == 64u8 => impl_load::<{ Register::R0 }>(ctx, opcode),
        op if op & 240u8 == 80u8 => impl_load::<{ Register::R1 }>(ctx, opcode),
        op if op & 240u8 == 96u8 => impl_load::<{ Register::R2 }>(ctx, opcode),
        op if op & 240u8 == 112u8 => impl_load::<{ Register::R3 }>(ctx, opcode),
        _ => {
            ::core::panicking::panic_fmt(
                format_args!("Unhandled opcode: 0x{0:02X}", opcode),
            );
        }
    }
}
```

## Features
- **`0` / `1`** - Fixed bits that must match
- **`_`** - Wildcard bits
- **`a-z`** - Variable bits (extracted and bound to const generics)
- **`'`** - Visual separator (ignored, for readability)
- `u8`, `u16`, `u32`, `u64` opcode sizes

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