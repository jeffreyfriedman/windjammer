//! Experimental AoSoA / SoA companion struct emission for cache locality hints.
//!
//! Enable with `WJ_EMIT_AOSOA_HINTS=1` when building; appends suggested Rust layouts to generated output.

use crate::analyzer::{AnalyzedFunction, AoSoACandidate};
use crate::codegen::rust::types::type_to_rust;
use crate::parser::{Item, Program, Type};
use std::collections::HashMap;

/// Append Rust `struct FooSoA { field: Vec<T>, ... }` sketches from analyzer hints.
pub fn emit_aosoa_hints<'ast>(
    program: &Program<'ast>,
    analyzed: &[AnalyzedFunction<'ast>],
) -> String {
    let layouts = collect_struct_fields(program);
    let mut out = String::new();
    for af in analyzed {
        for c in &af.cache_locality.aosoa_candidates {
            out.push_str(&emit_one_candidate(c, &layouts));
        }
    }
    out
}

fn collect_struct_fields(program: &Program<'_>) -> HashMap<String, Vec<(String, Type)>> {
    let mut m = HashMap::new();
    collect_struct_fields_rec(&program.items, &mut m);
    m
}

fn collect_struct_fields_rec(items: &[Item<'_>], m: &mut HashMap<String, Vec<(String, Type)>>) {
    for item in items {
        match item {
            Item::Struct { decl, .. } if !decl.is_extern && decl.tuple_fields.is_none() => {
                let v: Vec<_> = decl
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), f.field_type.clone()))
                    .collect();
                m.insert(decl.name.clone(), v);
            }
            Item::Mod { items: inner, .. } => collect_struct_fields_rec(inner, m),
            _ => {}
        }
    }
}

fn emit_one_candidate(
    c: &AoSoACandidate,
    layouts: &HashMap<String, Vec<(String, Type)>>,
) -> String {
    let mut s = String::new();
    let soa_name = format!("{}SoA", c.element_struct);
    s.push_str(&format!(
        "\n// WJ cache locality: SoA companion for `{}` (function `{}`, iterable `{}`, loop var `{}`)\n",
        c.element_struct, c.function_name, c.iterable_var, c.loop_var
    ));
    s.push_str(&format!(
        "// Hot fields (heuristic): {:?} | Cold: {:?} | SIMD-friendly layout: {}\n",
        c.hot_fields, c.cold_fields, c.simd_friendly_layout
    ));
    s.push_str("#[allow(dead_code)]\n");
    s.push_str("#[derive(Clone, Debug)]\n");
    s.push_str(&format!("pub struct {} {{\n", soa_name));
    if let Some(fields) = layouts.get(&c.element_struct) {
        for (fname, ty) in fields {
            let rust_ty = type_to_rust(ty);
            s.push_str(&format!(
                "    pub {}: Vec<{}>, // was `{}.{}\n",
                fname, rust_ty, c.element_struct, fname
            ));
        }
    } else {
        s.push_str("    // Struct not found in this unit — define SoA fields manually.\n");
    }
    s.push_str("}\n");

    let chunk = format!("{}AoSoA8", c.element_struct);
    s.push_str(
        "// AoSoA (8-wide chunks, DOTS-style): fixed SIMD lanes; iteration uses chunk × lane indexing.\n",
    );
    s.push_str("#[allow(dead_code)]\n");
    s.push_str("#[derive(Clone, Debug)]\n");
    s.push_str(&format!("pub struct {} {{\n", chunk));
    if let Some(fields) = layouts.get(&c.element_struct) {
        for (fname, ty) in fields {
            let rust_ty = type_to_rust(ty);
            s.push_str(&format!(
                "    pub {}: Vec<[{}; 8]>, // SoA lanes × {}\n",
                fname, rust_ty, fname
            ));
        }
        s.push_str("    pub len: usize,\n");
    }
    s.push_str("}\n");

    s.push_str(&format!(
        "// References: Unity DOTS chunk storage, Rust soa/soa-derive patterns, Mike Acton DOD talks.\n"
    ));
    s
}
