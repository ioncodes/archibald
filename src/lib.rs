//! This proc-macro allows defining instruction patterns with bit patterns like "1011xxyy"
//! and generates a dispatch function that calls const-generic handlers, enabling
//! compile-time branch folding.
//!
//! # Example
//! ```ignore
//! instruction_table! {
//!     type Opcode = u8;
//!
//!     dispatcher = dispatch;
//!     context = Cpu;
//!     
//!     // LDA immediate - pattern with variable addressing mode bits
//!     "101000mm" => Load<AddrMode::{mm}> where {
//!         mm: AddrMode = { 00 => Immediate, 01 => ZeroPage, 10 => Absolute, 11 => IndirectY }
//!     };
//!     
//!     // Fixed opcode
//!     "00011000" => Clc;
//! }
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, format_ident, quote};
use std::collections::HashMap;
use syn::{
    Ident, LitInt, LitStr, Token, Type, braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token,
};

/// A single bit mapping: 0b00 => R0 or 00 => R0
struct BitMapping {
    bits: String,
    variant: Ident,
}

impl Parse for BitMapping {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let bits_lit: LitInt = input.parse()?;
        let bits_str = bits_lit.to_string();

        // Handle both "0b00" and "00" formats, normalize to binary digits only
        let bits = if bits_str.starts_with("0b") || bits_str.starts_with("0B") {
            bits_str[2..].to_string()
        } else {
            // Assume it's already binary digits
            bits_str
        };

        input.parse::<Token![=>]>()?;
        let variant: Ident = input.parse()?;
        Ok(BitMapping { bits, variant })
    }
}

/// A variable binding: r: Register = { 0b00 => R0, ... } or r = { 0b00 => R0, ... }
struct VariableBinding {
    name: String,
    _enum_type: Option<Ident>,
    mappings: Vec<BitMapping>,
}

impl Parse for VariableBinding {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name_ident: Ident = input.parse()?;
        let name = name_ident.to_string();

        // Optional type annotation
        let enum_type = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        input.parse::<Token![=]>()?;

        let content;
        braced!(content in input);

        let mappings: Punctuated<BitMapping, Token![,]> =
            content.parse_terminated(BitMapping::parse, Token![,])?;

        Ok(VariableBinding {
            name,
            _enum_type: enum_type,
            mappings: mappings.into_iter().collect(),
        })
    }
}

/// A where clause with variable bindings
struct WhereClause {
    bindings: Vec<VariableBinding>,
}

impl Parse for WhereClause {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![where]>()?;
        let content;
        braced!(content in input);

        let bindings: Punctuated<VariableBinding, Token![,]> =
            content.parse_terminated(VariableBinding::parse, Token![,])?;

        Ok(WhereClause {
            bindings: bindings.into_iter().collect(),
        })
    }
}

/// Handler specification: Load<AddrMode::{mm}, Size::{ss}>
struct HandlerSpec {
    name: Ident,
    generics: Vec<GenericArg>,
}

enum GenericArg {
    /// A variable reference like AddrMode::{mm}
    Variable { enum_type: Ident, var_name: String },
    /// A fixed type/const
    Fixed(TokenStream2),
}

impl Parse for HandlerSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;

        let generics = if input.peek(Token![<]) {
            input.parse::<Token![<]>()?;
            let mut args = Vec::new();

            loop {
                if input.peek(Token![>]) {
                    break;
                }

                // Try to parse:
                // - EnumType::{var} (variable reference)
                // - EnumType::Variant (fixed path)
                // - { expr } (const expr in braces)
                // - Ident (simple type name)
                if input.peek(token::Brace) {
                    // { expr } - already braced const generic
                    let content;
                    braced!(content in input);
                    let inner: TokenStream2 = content.parse()?;
                    args.push(GenericArg::Fixed(quote! { { #inner } }));
                } else if input.peek(Ident) {
                    let first_ident: Ident = input.parse()?;

                    if input.peek(Token![::]) {
                        input.parse::<Token![::]>()?;

                        if input.peek(token::Brace) {
                            // EnumType::{var} - variable reference
                            let inner;
                            braced!(inner in input);
                            let var_ident: Ident = inner.parse()?;

                            args.push(GenericArg::Variable {
                                enum_type: first_ident,
                                var_name: var_ident.to_string(),
                            });
                        } else {
                            // EnumType::Variant - fixed path
                            let variant: Ident = input.parse()?;
                            let path = quote! { #first_ident :: #variant };
                            args.push(GenericArg::Fixed(path));
                        }
                    } else {
                        // Just an identifier
                        args.push(GenericArg::Fixed(first_ident.to_token_stream()));
                    }
                } else {
                    // Parse as arbitrary tokens until comma or >
                    let mut tokens = TokenStream2::new();
                    while !input.peek(Token![,]) && !input.peek(Token![>]) {
                        let tt: proc_macro2::TokenTree = input.parse()?;
                        tokens.extend(std::iter::once(tt));
                    }
                    args.push(GenericArg::Fixed(tokens));
                }

                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                } else {
                    break;
                }
            }

            input.parse::<Token![>]>()?;
            args
        } else {
            Vec::new()
        };

        Ok(HandlerSpec { name, generics })
    }
}

/// A single instruction pattern entry
struct InstructionEntry {
    pattern: String,
    handler: HandlerSpec,
    where_clause: Option<WhereClause>,
}

impl Parse for InstructionEntry {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let pattern_lit: LitStr = input.parse()?;
        let pattern = pattern_lit.value();

        input.parse::<Token![=>]>()?;

        let handler: HandlerSpec = input.parse()?;

        let where_clause = if input.peek(Token![where]) {
            Some(input.parse()?)
        } else {
            None
        };

        Ok(InstructionEntry {
            pattern,
            handler,
            where_clause,
        })
    }
}

/// The full instruction table definition
struct InstructionTable {
    opcode_type: Type,
    dispatcher_name: Ident,
    context_type: Type,
    entries: Vec<InstructionEntry>,
}

impl Parse for InstructionTable {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // type Opcode = u8;
        input.parse::<Token![type]>()?;
        let _: Ident = input.parse()?; // "Opcode"
        input.parse::<Token![=]>()?;
        let opcode_type: Type = input.parse()?;
        input.parse::<Token![;]>()?;

        // dispatcher = dispatch;
        let _: Ident = input.parse()?; // "dispatcher"
        input.parse::<Token![=]>()?;
        let dispatcher_name: Ident = input.parse()?;
        input.parse::<Token![;]>()?;

        // context = Cpu;
        let _: Ident = input.parse()?; // "context"
        input.parse::<Token![=]>()?;
        let context_type: Type = input.parse()?;
        input.parse::<Token![;]>()?;

        // Parse instruction entries
        let mut entries = Vec::new();
        while !input.is_empty() {
            entries.push(input.parse()?);
            input.parse::<Token![;]>()?;
        }

        Ok(InstructionTable {
            opcode_type,
            dispatcher_name,
            context_type,
            entries,
        })
    }
}

/// Represents a parsed bit pattern
#[derive(Debug, Clone)]
struct ParsedPattern {
    /// Fixed bits mask (1 = fixed, 0 = variable or wildcard)
    mask: u64,
    /// Expected values for fixed bits
    value: u64,
    /// Map from variable name to (start_bit, num_bits)
    variables: HashMap<String, (u8, u8)>,
    /// Wildcard bit positions (e.g. immediates)
    wildcard_bits: u64,
    /// Number of bits in the pattern (8, 16, 32, or 64)
    bit_width: usize,
}

fn parse_pattern(pattern: &str) -> ParsedPattern {
    let pattern = pattern.trim();
    let bit_width = pattern.len();

    assert!(
        bit_width == 8 || bit_width == 16 || bit_width == 32 || bit_width == 64,
        "Pattern must be exactly 8, 16, 32, or 64 bits. Got {} bits: {}",
        bit_width,
        pattern
    );

    let mut mask = 0u64;
    let mut value = 0u64;
    let mut wildcard_bits = 0u64;
    let mut var_positions: HashMap<char, Vec<u8>> = HashMap::new();

    for (i, ch) in pattern.chars().enumerate() {
        let bit_pos = (bit_width - 1 - i) as u8; // MSB first

        match ch {
            '0' => {
                mask |= 1 << bit_pos;
                // value bit stays 0
            }
            '1' => {
                mask |= 1 << bit_pos;
                value |= 1 << bit_pos;
            }
            '_' | '.' => {
                // Wildcard - don't care bit (not in mask, not a variable)
                wildcard_bits |= 1 << bit_pos;
            }
            c if c.is_ascii_lowercase() => {
                var_positions.entry(c).or_default().push(bit_pos);
            }
            _ => panic!(
                "Invalid pattern character: '{}'. Use 0/1 for fixed bits, a-z for variables, _ or . for wildcards",
                ch
            ),
        }
    }

    // Convert var positions to (start_bit, num_bits)
    let mut variables = HashMap::new();
    for (var_char, positions) in var_positions {
        let min_pos = *positions.iter().min().unwrap();
        let num_bits = positions.len() as u8;
        variables.insert(var_char.to_string(), (min_pos, num_bits));
    }

    ParsedPattern {
        mask,
        value,
        variables,
        wildcard_bits,
        bit_width,
    }
}

/// Generate all possible opcodes matching a pattern with variable substitutions
fn generate_opcode_variants(
    pattern: &ParsedPattern,
    bindings: &[VariableBinding],
) -> Vec<(u64, Vec<(String, Ident, Ident)>)> {
    // Build a list of (var_name, bit_pos, num_bits, mappings)
    let mut var_info: Vec<(&str, u8, u8, &[BitMapping])> = Vec::new();

    for binding in bindings {
        if let Some(&(bit_pos, num_bits)) = pattern.variables.get(&binding.name) {
            var_info.push((&binding.name, bit_pos, num_bits, &binding.mappings));
        }
    }

    if var_info.is_empty() {
        // No variables, just return the base pattern
        return vec![(pattern.value, vec![])];
    }

    // Generate all combinations
    let mut results = Vec::new();
    generate_combinations(pattern.value, &var_info, 0, vec![], &mut results);

    results
}

fn generate_combinations(
    current_opcode: u64,
    var_info: &[(&str, u8, u8, &[BitMapping])],
    index: usize,
    current_bindings: Vec<(String, Ident, Ident)>,
    results: &mut Vec<(u64, Vec<(String, Ident, Ident)>)>,
) {
    if index >= var_info.len() {
        results.push((current_opcode, current_bindings));
        return;
    }

    let (var_name, bit_pos, _num_bits, mappings) = &var_info[index];

    for mapping in *mappings {
        let bits_value = u64::from_str_radix(&mapping.bits, 2).expect("Invalid binary string");
        let new_opcode = current_opcode | (bits_value << bit_pos);

        let mut new_bindings = current_bindings.clone();
        new_bindings.push((
            var_name.to_string(),
            format_ident!("{}", var_name),
            mapping.variant.clone(),
        ));

        generate_combinations(new_opcode, var_info, index + 1, new_bindings, results);
    }
}

fn make_literal(value: u64, bit_width: usize) -> proc_macro2::Literal {
    match bit_width {
        8 => proc_macro2::Literal::u8_suffixed(value as u8),
        16 => proc_macro2::Literal::u16_suffixed(value as u16),
        32 => proc_macro2::Literal::u32_suffixed(value as u32),
        64 => proc_macro2::Literal::u64_suffixed(value),
        _ => panic!("Unsupported bit width: {}", bit_width),
    }
}

fn make_full_mask(bit_width: usize) -> u64 {
    match bit_width {
        8 => 0xFF,
        16 => 0xFFFF,
        32 => 0xFFFF_FFFF,
        64 => 0xFFFF_FFFF_FFFF_FFFF,
        _ => panic!("Unsupported bit width: {}", bit_width),
    }
}

fn generate_handler_call(
    handler: &HandlerSpec,
    bindings: &[(String, Ident, Ident)],
    _where_clause: &Option<WhereClause>,
) -> TokenStream2 {
    let handler_name = &handler.name;

    let generic_args: Vec<TokenStream2> = handler
        .generics
        .iter()
        .map(|arg| match arg {
            GenericArg::Variable {
                enum_type,
                var_name,
            } => {
                // Find the variant for this variable
                let variant = bindings
                    .iter()
                    .find(|(name, _, _)| name == var_name)
                    .map(|(_, _, variant)| variant)
                    .unwrap_or_else(|| {
                        panic!("Variable '{}' not found in where clause bindings", var_name)
                    });
                // Wrap in braces for const generic
                quote! { { #enum_type::#variant } }
            }
            GenericArg::Fixed(tokens) => {
                // Check if already wrapped in braces
                let s = tokens.to_string();
                if s.starts_with('{') {
                    tokens.clone()
                } else {
                    // Wrap in braces for const generic
                    quote! { { #tokens } }
                }
            }
        })
        .collect();

    if generic_args.is_empty() {
        quote! { #handler_name(ctx, opcode) }
    } else {
        quote! { #handler_name::<#(#generic_args),*>(ctx, opcode) }
    }
}

#[proc_macro]
pub fn instruction_table(input: TokenStream) -> TokenStream {
    let table = parse_macro_input!(input as InstructionTable);

    let opcode_type = &table.opcode_type;
    let dispatcher_name = &table.dispatcher_name;
    let context_type = &table.context_type;

    // Collect all match arms
    let mut match_arms = Vec::new();
    let mut seen_patterns: Vec<(u64, u64)> = Vec::new(); // (mask, value) pairs

    for entry in &table.entries {
        let pattern = parse_pattern(&entry.pattern);
        let bit_width = pattern.bit_width;
        let bindings = entry
            .where_clause
            .as_ref()
            .map(|wc| wc.bindings.as_slice())
            .unwrap_or(&[]);

        // Check if pattern has variables that need expansion
        let has_expandable_vars = bindings
            .iter()
            .any(|b| pattern.variables.contains_key(&b.name));

        if has_expandable_vars {
            // Pattern with variables, expand all combinations
            let variants = generate_opcode_variants(&pattern, bindings);

            // Calculate the combined mask: fixed bits + variable bits that are expanded
            let mut expanded_mask = pattern.mask;
            for binding in bindings {
                if let Some(&(bit_pos, num_bits)) = pattern.variables.get(&binding.name) {
                    // Add the variable bits to the mask since we're expanding them
                    for i in 0..num_bits {
                        expanded_mask |= 1 << (bit_pos + i);
                    }
                }
            }

            for (opcode, var_bindings) in variants {
                // For patterns with wildcards, we need a range match
                if pattern.wildcard_bits != 0 {
                    // Generate a guard-based match
                    let handler_call =
                        generate_handler_call(&entry.handler, &var_bindings, &entry.where_clause);

                    // Check if this pattern overlaps with existing ones
                    let full_mask = make_full_mask(bit_width);
                    let dominated = seen_patterns
                        .iter()
                        .any(|(m, v)| *m == full_mask && *v == opcode);

                    if !dominated {
                        let mask_lit = make_literal(expanded_mask, bit_width);
                        let value_lit = make_literal(opcode, bit_width);
                        match_arms.push(quote! {
                            op if op & #mask_lit == #value_lit => { #handler_call }
                        });
                        seen_patterns.push((expanded_mask, opcode));
                    }
                } else {
                    // Exact match
                    let full_mask = make_full_mask(bit_width);
                    let key = (full_mask, opcode);
                    if !seen_patterns.contains(&key) {
                        let handler_call = generate_handler_call(
                            &entry.handler,
                            &var_bindings,
                            &entry.where_clause,
                        );
                        let opcode_lit = make_literal(opcode, bit_width);
                        match_arms.push(quote! {
                            #opcode_lit => { #handler_call }
                        });
                        seen_patterns.push(key);
                    }
                }
            }
        } else if pattern.wildcard_bits != 0 || !pattern.variables.is_empty() {
            // Pattern with wildcards but no where clause bindings. single masked match
            let mask = pattern.mask;
            let value = pattern.value;
            let handler_call = generate_handler_call(&entry.handler, &[], &entry.where_clause);

            let mask_lit = make_literal(mask, bit_width);
            let value_lit = make_literal(value, bit_width);
            match_arms.push(quote! {
                op if op & #mask_lit == #value_lit => { #handler_call }
            });
            seen_patterns.push((mask, value));
        } else {
            // Fixed pattern, exact match
            let opcode = pattern.value;
            let full_mask = make_full_mask(bit_width);
            let key = (full_mask, opcode);
            if !seen_patterns.contains(&key) {
                let handler_call = generate_handler_call(&entry.handler, &[], &entry.where_clause);
                let opcode_lit = make_literal(opcode, bit_width);
                match_arms.push(quote! {
                    #opcode_lit => { #handler_call }
                });
                seen_patterns.push(key);
            }
        }
    }

    // Generate the dispatcher function
    let expanded = quote! {
        #[inline]
        pub fn #dispatcher_name(ctx: &mut #context_type, opcode: #opcode_type) {
            match opcode {
                #(#match_arms)*
                _ => panic!("Unhandled opcode: 0x{:02X}", opcode),
            }
        }
    };

    TokenStream::from(expanded)
}
