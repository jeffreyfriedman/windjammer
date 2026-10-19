//! Typed JavaScript/TypeScript emitter over the portable IR.
//!
//! 64-bit integers are BigInt, narrower integers are range-checked Numbers, structs are classes
//! with explicit `clone()` (value semantics are inserted by lowering as `PK::Copy`), enums are
//! tagged variant classes, Option is `null | {v}`, Result is `{ok, v}`.

mod builtin;
mod enumerable;
mod expr;
mod fmt;
mod items;
mod names;
mod num;
mod ordering;
mod pat;
mod refs;
mod stmt;
mod types;

use crate::codegen::portable::ir::*;
use std::collections::HashSet;

pub struct EmitOptions {
    /// Emit TypeScript annotations (`.ts`, run with Node's type stripping or `tsc`).
    pub typescript: bool,
}

pub(super) struct Js<'m> {
    pub m: &'m PModule,
    pub ts: bool,
    pub out: String,
    pub ind: usize,
    pub tmp: usize,
    /// Every struct field name in the module (methods with the same name are mangled).
    pub fields: HashSet<String>,
    /// A `'label:` seen by `Labeled`, claimed by the next emitted `for`/`while`.
    pub pending_label: Option<String>,
    /// Locals that are `&mut` accessor cells (`x.v` reads/writes the borrowed place); see `refs.rs`.
    pub cells: HashSet<String>,
    /// Locals aliasing an object-represented value through `&mut` (`*x = v` replaces in place).
    pub obj_aliases: HashSet<String>,
}

/// Emit the program module (`main.mjs` / `main.ts`), importing the runtime from `./wj_core.mjs`.
pub fn emit_module(m: &PModule, opts: &EmitOptions) -> PResult<String> {
    let mut js = Js { m, ts: opts.typescript, out: String::new(), ind: 0, tmp: 0, fields: HashSet::new(), pending_label: None, cells: HashSet::new(), obj_aliases: HashSet::new() };
    for s in &m.structs {
        s.fields.iter().for_each(|(n, _)| {
            js.fields.insert(n.clone());
        });
    }
    js.module()?;
    Ok(js.out)
}

impl<'m> Js<'m> {
    pub fn line(&mut self, s: &str) {
        for _ in 0..self.ind {
            self.out.push_str("  ");
        }
        self.out.push_str(s);
        self.out.push('\n');
    }

    pub fn fresh(&mut self, base: &str) -> String {
        self.tmp += 1;
        format!("${base}{}", self.tmp)
    }

    pub fn unsupported<T>(&self, what: impl Into<String>) -> PResult<T> {
        Err(Unsupported { what: what.into(), line: 0 })
    }
}
