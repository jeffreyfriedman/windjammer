//! Mutation self-test: inject known backend bugs into generated artifacts and require the
//! harness to catch every one. Mutations are text edits on the emitted files only (no compiler
//! flags), so each mutant models a specific class of backend defect.

use super::backends::Backend;
use std::path::Path;

pub enum Edit {
    /// Replace the first-and-only exact occurrence in `file`.
    Replace { file: &'static str, from: &'static str, to: &'static str },
    /// Replace every occurrence in `file`.
    ReplaceAll { file: &'static str, from: &'static str, to: &'static str },
    /// Replace every line containing `contains` with `to`.
    Lines { file: &'static str, contains: &'static str, to: &'static str },
}

pub struct Mutant {
    pub name: &'static str,
    pub models: &'static str,
    pub backend: Backend,
    pub edits: Vec<Edit>,
    /// Corpus programs expected to expose the defect (any one failing kills the mutant).
    pub programs: &'static [&'static str],
}

pub fn apply(dir: &Path, edits: &[Edit]) -> Result<(), String> {
    for e in edits {
        match e {
            Edit::Replace { file, from, to } => {
                let p = dir.join(file);
                let text = std::fs::read_to_string(&p).map_err(|e| format!("{file}: {e}"))?;
                if !text.contains(from) {
                    return Err(format!("{file} does not contain `{from}`"));
                }
                std::fs::write(&p, text.replacen(from, to, 1)).map_err(|e| e.to_string())?;
            }
            Edit::ReplaceAll { file, from, to } => {
                let p = dir.join(file);
                let text = std::fs::read_to_string(&p).map_err(|e| format!("{file}: {e}"))?;
                if !text.contains(from) {
                    return Err(format!("{file} does not contain `{from}`"));
                }
                std::fs::write(&p, text.replace(from, to)).map_err(|e| e.to_string())?;
            }
            Edit::Lines { file, contains, to } => {
                let p = dir.join(file);
                let text = std::fs::read_to_string(&p).map_err(|e| format!("{file}: {e}"))?;
                let mut hits = 0;
                let out: Vec<String> = text
                    .lines()
                    .map(|l| {
                        if l.contains(contains) {
                            hits += 1;
                            to.to_string()
                        } else {
                            l.to_string()
                        }
                    })
                    .collect();
                if hits == 0 {
                    return Err(format!("{file} has no line containing `{contains}`"));
                }
                std::fs::write(&p, out.join("\n")).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

/// The seeded list. Documented in docs/backends/CONFORMANCE.md; keep the two in sync.
pub fn mutants() -> Vec<Mutant> {
    vec![
        Mutant {
            name: "js-i64-via-f64",
            models: "i64 arithmetic computed through f64 (precision loss above 2^53)",
            backend: Backend::Js,
            edits: vec![Edit::Replace { file: "wj_core.mjs", from: "export const $i64 = (r) => (r >= -9223372036854775808n && r <= 9223372036854775807n ? r : ovf());", to: "export const $i64 = (r) => BigInt(Number(r));" }],
            programs: &["04_int_overflow_i64_large", "46_modpow_i64"],
        },
        Mutant {
            name: "js-struct-copy-by-reference",
            models: "struct assignment aliases instead of copying",
            backend: Backend::Js,
            edits: vec![
                Edit::Lines { file: "main.mjs", contains: "clone() { return new ", to: "  clone() { return this; }" },
                Edit::Replace { file: "wj_core.mjs", from: "  if (typeof x.clone === \"function\") return x.clone();", to: "  if (typeof x.clone === \"function\") return x;" },
            ],
            programs: &["10_struct_copy_semantics", "43_struct_value_semantics"],
        },
        Mutant {
            name: "js-int-division-as-float",
            models: "integer division not truncated",
            backend: Backend::Js,
            edits: vec![Edit::Replace { file: "wj_core.mjs", from: "Math.trunc(a / b)", to: "a / b" }],
            programs: &["01_arith_i32"],
        },
        Mutant {
            name: "js-u64-as-signed",
            models: "u64 wrapping implemented as signed",
            backend: Backend::Js,
            edits: vec![Edit::ReplaceAll { file: "main.mjs", from: "BigInt.asUintN(64,", to: "BigInt.asIntN(64," }],
            programs: &["40_prng_fnv_checksum"],
        },
        Mutant {
            name: "js-no-overflow-check",
            models: "i32 overflow silently ignored",
            backend: Backend::Js,
            edits: vec![Edit::Replace { file: "wj_core.mjs", from: "export const $i32 = (r) => (r >= -2147483648 && r <= 2147483647 ? r : ovf());", to: "export const $i32 = (r) => r;" }],
            programs: &["60_overflow_panics"],
        },
        Mutant {
            name: "go-i64-as-i32",
            models: "64-bit integers narrowed to 32 bits",
            backend: Backend::Go,
            edits: vec![Edit::ReplaceAll { file: "main.go", from: "int64", to: "int32" }],
            programs: &["02_arith_i64", "46_modpow_i64"],
        },
        Mutant {
            name: "go-u64-as-i64",
            models: "unsigned 64-bit treated as signed",
            backend: Backend::Go,
            edits: vec![Edit::ReplaceAll { file: "main.go", from: "uint64", to: "int64" }],
            programs: &["40_prng_fnv_checksum", "45_string_ops"],
        },
        Mutant {
            name: "go-clone-shares-backing-store",
            models: "deep copy replaced by shallow copy (aliasing)",
            backend: Backend::Go,
            edits: vec![Edit::Replace { file: "wj_core.go", from: "return wjCopyValue(reflect.ValueOf(&x).Elem()).Interface().(T)", to: "return x" }],
            programs: &["11_struct_clone_independent", "43_struct_value_semantics"],
        },
        Mutant {
            name: "go-no-overflow-check",
            models: "integer overflow wraps silently",
            backend: Backend::Go,
            edits: vec![Edit::Replace { file: "wj_core.go", from: "	c, o := addC(a, b)\n	if o {\n		wjOverflow(\"add\")\n	}\n	return c", to: "	c, _ := addC(a, b)\n	return c" }],
            programs: &["60_overflow_panics"],
        },
    ]
}
