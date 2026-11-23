//! Gates of the table-driven std surface (docs/design/table-driven-std-surface.md, section 4): the lowering rows
//! on the declarations of `std/rust_core.wji` are complete, well-formed and agree with everything else that knows
//! the same method.
//!
//! * G1  every declaration of a migrated type has, per target, a rule or an `unsupported` reason; templates parse,
//!       name only declared parameters, never use `${`; a `when` list ends in an unguarded rule.
//! * G4  interpreter parity: the rows against the interpreter's registry, both directions; `&mut self` against the
//!       registration's mutability.
//! * G5  helper existence: every helper a template calls is defined in the runtime shim of its target, and no helper
//!       of the shim is dead.
//! * G6  vocabulary cap: how many variants `Guard`, `Hook`, `Modifier`, `Fact` (and the other closed enums) have.
//! * G8  ADR-0039 cross-check: a declaration named like a `removed` form needs `@internal(reason)`.
//!
//! G2 is `tests/portable_emit_golden_test.rs`, G3 is `tests/std_row_examples_test.rs`, G7 is
//! `tests/heuristic_surface_test.rs`. The gates read `std/rust_core.wji` and the shims from disk, so a planted
//! mistake is seen without rebuilding the compiler (`embedded_surface_is_the_file_on_disk` ties the two together).

use std::collections::BTreeSet;
use std::path::PathBuf;
use windjammer::codegen::portable::template::{Hole, Template};
use windjammer::interpreter::builtins::{Kind, Registry};
use windjammer::stdlib_surface::lowering::{CoreType, Fact, Guard, Hook, LoweringTable, Modifier, RawTrait, Rule, Target};
use windjammer::typing::core_surface::CoreSurface;

/// The types whose methods are migrated: every declaration of these traits needs a rule per target. A stage adds its
/// types here (and to `CoreType`); the list never shrinks.
const MIGRATED: &[CoreType] = &[CoreType::Float, CoreType::Integer];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn surface() -> CoreSurface {
    CoreSurface::parse_source(&read("std/rust_core.wji"))
}

fn table_of(core: &CoreSurface) -> LoweringTable {
    LoweringTable::from_declarations(&core.declared_traits()).unwrap_or_else(|e| panic!("lowering decorators of std/rust_core.wji: {e}"))
}

fn table() -> LoweringTable {
    table_of(&surface())
}

fn declared(core: &CoreSurface, ty: CoreType) -> Vec<String> {
    let traits: Vec<RawTrait> = core.declared_traits();
    traits.into_iter().filter(|t| t.name == ty.trait_name()).flat_map(|t| t.methods.into_iter().map(|m| m.name)).collect()
}

#[test]
fn embedded_surface_is_the_file_on_disk() {
    assert_eq!(include_str!("../std/rust_core.wji"), read("std/rust_core.wji"), "the gates read the file; the compiler embeds it");
}

// ---------------------------------------------------------------------------------------------
// G1: completeness and well-formedness
// ---------------------------------------------------------------------------------------------

/// Every problem of the rows of one table, as text (empty when the table is well-formed).
fn row_problems(core: &CoreSurface, table: &LoweringTable) -> Vec<String> {
    let mut bad = Vec::new();
    for ty in MIGRATED {
        let methods = declared(core, *ty);
        if methods.is_empty() {
            bad.push(format!("{}: no declared methods", ty.trait_name()));
        }
        for m in &methods {
            let id = format!("{}::{m}", ty.trait_name());
            let Some(row) = table.row(*ty, m) else {
                bad.push(format!("{id}: no row"));
                continue;
            };
            for target in Target::ALL {
                let rules = row.rules(target);
                if rules.is_empty() {
                    bad.push(format!("{id}: no `@{}` rule (give it `emit`, `hook` or `unsupported = \"reason\"`)", target.decorator()));
                    continue;
                }
                if let Some(last) = rules.last() {
                    if matches!(last, Rule::Emit { when, .. } if !when.is_empty()) {
                        bad.push(format!("{id}: the last `@{}` rule is guarded; a `when` list ends with an unguarded rule", target.decorator()));
                    }
                }
                for rule in rules {
                    match rule {
                        Rule::Unsupported(reason) if reason.trim().is_empty() => bad.push(format!("{id}: empty `unsupported` reason on `@{}`", target.decorator())),
                        Rule::Emit { template, .. } => {
                            if template.contains("${") {
                                bad.push(format!("{id}: `${{` in template `{template}`"));
                            }
                            match Template::parse(template) {
                                Err(e) => bad.push(format!("{id}: {e}")),
                                Ok(t) => {
                                    for h in t.holes() {
                                        if let Hole::Arg(name, _) = h {
                                            if !row.params.contains(name) {
                                                bad.push(format!("{id}: `{{{name}}}` is not a declared parameter (declared: {:?})", row.params));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            match &row.example {
                None => bad.push(format!("{id}: no `@example` (G3 builds its program from it)")),
                Some(e) if e.src.trim().is_empty() || e.expect.trim().is_empty() => bad.push(format!("{id}: empty `@example`")),
                _ => {}
            }
        }
    }
    bad
}

#[test]
fn every_migrated_declaration_has_a_well_formed_rule_per_target() {
    let core = surface();
    let bad = row_problems(&core, &table_of(&core));
    assert!(bad.is_empty(), "lowering rows of std/rust_core.wji:\n{}", bad.join("\n"));
}

#[test]
fn every_core_type_is_migrated_and_listed_once() {
    let listed: BTreeSet<CoreType> = MIGRATED.iter().copied().collect();
    assert_eq!(listed.len(), MIGRATED.len(), "MIGRATED lists a type twice");
    assert_eq!(listed, CoreType::ALL.iter().copied().collect::<BTreeSet<_>>(), "a CoreType whose rows are not checked by G1");
}

#[test]
fn the_row_checker_sees_a_planted_mistake() {
    // The gate's own oracle: the same checker over doctored copies of the file must report each mistake.
    let text = read("std/rust_core.wji");
    let plant = |from: &str, to: &str| -> Vec<String> {
        assert!(text.contains(from), "plant: `{from}` not in std/rust_core.wji");
        let core = CoreSurface::parse_source(&text.replacen(from, to, 1));
        row_problems(&core, &table_of(&core))
    };
    assert!(plant("@go(emit = \"wjMath_sqrt({self:f64})\")", "").iter().any(|p| p.contains("Float::sqrt") && p.contains("no `@go` rule")));
    assert!(plant("Math.pow({self}, {n})", "Math.pow({self}, {indx})").iter().any(|p| p.contains("{indx}")));
    assert!(plant("@go(emit = \"wjMath_sqrt({self:f64})\")", "@go(unsupported = \"\")").iter().any(|p| p.contains("empty `unsupported` reason")));
    // `${` inside a string is an interpolation to the parser, which the loader refuses outright
    let interpolated = std::panic::catch_unwind(|| plant("Math.cbrt({self})", "Math.cbrt(${self})"));
    assert!(interpolated.is_err(), "a `${{` template must not load");
    assert!(plant("@example(src = \"(16.0).sqrt()\", expect = \"4\")", "").iter().any(|p| p.contains("no `@example`")));
    // the Integer rows are held to the same rules
    assert!(plant("@go(emit = \"wjAbs({self})\")", "").iter().any(|p| p.contains("Integer::abs") && p.contains("no `@go` rule")));
    assert!(plant("$min2({self}, {other})", "$min2({self}, {othr})").iter().any(|p| p.contains("{othr}")));
    assert!(plant("@js(emit = \"$max2({self}, {other})\")", "@js(emit = \"$max2({self}, {other})\", when = \"t_wide\")").iter().any(|p| p.contains("Integer::max") && p.contains("guarded")));
    assert!(plant("@js(emit = \"$sat({T.min}, {T.max}, {self} + {rhs})\")", "@js(unsupported = \"\")").iter().any(|p| p.contains("empty `unsupported` reason")));
    assert!(plant("{T.chk}({self} ** {exp})", "{T.chk}({self} ** {exp}){T.nope}").iter().any(|p| p.contains("unknown fact")));
}

// ---------------------------------------------------------------------------------------------
// G4: interpreter parity (Float, Integer)
// ---------------------------------------------------------------------------------------------

/// The interpreter's receiver kind of a migrated type.
fn kind_of(ty: CoreType) -> Kind {
    match ty {
        CoreType::Float => Kind::Float,
        CoreType::Integer => Kind::Int,
    }
}

/// Methods the interpreter registers for a type that its trait does not declare yet. Each list may only shrink: declare
/// the method (with rules) and delete the entry. Float: `to_bits`, `to_le_bytes` and `to_be_bytes` are declared on the
/// width traits `F32` / `F64`, whose rows are not written yet; `total_cmp` needs `Ordering`. Integer: the byte
/// conversions are the width traits' too, `overflowing_*` return tuples and `next_power_of_two` needs the unsigned
/// types alone, and the ASCII methods of a `u8` belong with the char stage.
const REGISTERED_WITHOUT_ROW: &[(CoreType, &[&str])] = &[
    (CoreType::Float, &["copysign", "cosh", "exp2", "log", "recip", "rem_euclid", "sinh", "tanh", "to_be_bytes", "to_bits", "to_le_bytes", "total_cmp"]),
    (
        CoreType::Integer,
        &["is_ascii_alphabetic", "is_ascii_digit", "is_ascii_whitespace", "next_power_of_two", "overflowing_add", "overflowing_mul", "to_ascii_lowercase", "to_ascii_uppercase", "to_be_bytes", "to_le_bytes"],
    ),
];

#[test]
fn every_row_is_registered_in_the_interpreter_with_the_same_mutability() {
    let core = surface();
    let table = table_of(&core);
    let mut bad = Vec::new();
    for ty in MIGRATED {
        let registered = Registry::standard().methods_of(kind_of(*ty));
        for (_, row) in table.rows().filter(|(t, _)| t == ty) {
            let id = format!("{}::{}", ty.trait_name(), row.name);
            match registered.iter().find(|(n, _)| *n == row.name) {
                None => bad.push(format!("{id}: declared, but the interpreter registers no {} method of that name", ty.noun())),
                Some((_, mutating)) if *mutating != row.changes_receiver => {
                    bad.push(format!("{id}: declared {}, registered {}", if row.changes_receiver { "`&mut self`" } else { "`self`" }, if *mutating { "mutating" } else { "pure" }))
                }
                _ => {}
            }
        }
    }
    assert!(bad.is_empty(), "rows against the interpreter registry:\n{}", bad.join("\n"));
}

#[test]
fn every_registered_method_is_a_row_or_on_the_shrinking_list() {
    let table = table();
    let mut bad = Vec::new();
    for ty in MIGRATED {
        let registered = Registry::standard().methods_of(kind_of(*ty));
        let listed_for: &[&str] = REGISTERED_WITHOUT_ROW.iter().find(|(t, _)| t == ty).map_or(&[], |(_, l)| l);
        for (name, _) in &registered {
            let is_row = table.row(*ty, name).is_some();
            let listed = listed_for.contains(name);
            if !is_row && !listed {
                bad.push(format!("the interpreter registers the {} method `{name}`, which `trait {}` does not declare (declare it with rules, or mark it `@internal`)", ty.noun(), ty.trait_name()));
            }
            if is_row && listed {
                bad.push(format!("`{name}` is a row now: delete it from REGISTERED_WITHOUT_ROW ({})", ty.trait_name()));
            }
        }
        for name in listed_for {
            if !registered.iter().any(|(n, _)| n == name) {
                bad.push(format!("`{name}` is on the {} list of REGISTERED_WITHOUT_ROW but the interpreter no longer registers it: delete it", ty.trait_name()));
            }
        }
    }
    assert!(bad.is_empty(), "interpreter registry against the rows:\n{}", bad.join("\n"));
}

// ---------------------------------------------------------------------------------------------
// G5: helper existence
// ---------------------------------------------------------------------------------------------

/// Identifiers a template of this target calls: JavaScript `$name`, Go `wjName` / `wjMath_name`.
fn helpers_in(text: &str, target: Target) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let starts = match target {
            Target::Js => c == '$' && chars.get(i + 1).is_some_and(|n| n.is_alphabetic()),
            Target::Go => c == 'w' && chars.get(i + 1) == Some(&'j') && chars.get(i + 2).is_some_and(|n| n.is_uppercase() || *n == '_' || *n == 'M') && (i == 0 || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '_')),
        };
        if starts {
            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            found.insert(chars[i..j].iter().collect());
            i = j;
        } else {
            i += 1;
        }
    }
    found
}

/// The helpers a shim defines: JavaScript `function $name` / `const $name`, Go `func wjName(` / `var|const wjName`.
fn defined_helpers(files: &[&str], target: Target) -> BTreeSet<String> {
    let mut defined = BTreeSet::new();
    for f in files {
        for line in read(f).lines() {
            let line = line.trim_start();
            let rest = match target {
                Target::Js => line.strip_prefix("export ").unwrap_or(line).strip_prefix("function ").or_else(|| line.strip_prefix("export ").unwrap_or(line).strip_prefix("const ")),
                Target::Go => line.strip_prefix("func ").or_else(|| line.strip_prefix("var ")).or_else(|| line.strip_prefix("const ")),
            };
            if let Some(rest) = rest {
                let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$').collect();
                if !name.is_empty() {
                    defined.insert(name);
                }
            }
        }
    }
    defined
}

fn shim_files(target: Target) -> &'static [&'static str] {
    match target {
        Target::Js => &["std/runtime/js/wj_core.mjs", "std/runtime/js/wj_std.mjs"],
        Target::Go => &["std/runtime/go/wj_core.go", "std/runtime/go/wj_std.go"],
    }
}

fn row_templates(table: &LoweringTable, target: Target) -> Vec<(String, String)> {
    table
        .rows()
        .flat_map(|(ty, row)| {
            row.rules(target).iter().filter_map(move |r| match r {
                Rule::Emit { template, .. } => Some((format!("{}::{}", ty.trait_name(), row.name), template.clone())),
                _ => None,
            })
        })
        .collect()
}

#[test]
fn every_helper_a_template_calls_is_defined_in_the_shim() {
    let table = table();
    let mut bad = Vec::new();
    for target in Target::ALL {
        let defined = defined_helpers(shim_files(target), target);
        assert!(defined.len() > 20, "{target:?}: the shim scan found only {} helpers", defined.len());
        for (id, template) in row_templates(&table, target) {
            for h in helpers_in(&template, target) {
                if !defined.contains(&h) {
                    bad.push(format!("{id}: `@{}` calls `{h}`, which {} does not define", target.decorator(), shim_files(target).join(" / ")));
                }
            }
        }
    }
    assert!(bad.is_empty(), "undefined helpers:\n{}", bad.join("\n"));
}

/// Helpers a shim defines that neither a row nor the Rust source of the backend (`src/codegen`) names. Shrink-only: a
/// helper that nothing calls is deleted, not listed. (Measured at stage 1: none.)
const DEAD_HELPERS_JS: &[&str] = &[];
/// Helpers the emitters name by building the text (`$std_<module>_<fn>`, `wjstd_<module>_<fn>` for the module
/// functions of `portable/stdlib.rs`): no literal names them, so a text search cannot tell them dead. The module-function
/// stage (7) turns them into rows and removes this exemption.
const DYNAMIC_PREFIXES: &[&str] = &["$std_", "$caps_", "wjstd_", "wjcaps_"];
const DEAD_HELPERS_GO: &[&str] = &[];

#[test]
fn no_shim_helper_is_dead() {
    let table = table();
    let mut sources = String::new();
    let mut stack = vec![root().join("src/codegen")];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                sources.push_str(&std::fs::read_to_string(&p).unwrap_or_default());
            }
        }
    }
    let mut bad = Vec::new();
    for (target, allowed) in [(Target::Js, DEAD_HELPERS_JS), (Target::Go, DEAD_HELPERS_GO)] {
        let used_in_rows: BTreeSet<String> = row_templates(&table, target).iter().flat_map(|(_, t)| helpers_in(t, target)).collect();
        let used_in_rust = helpers_in(&sources, target);
        let shim: String = shim_files(target).iter().map(|f| read(f)).collect::<Vec<_>>().join("\n");
        for h in defined_helpers(shim_files(target), target) {
            // a helper the shim's own code calls is alive through its caller; only the entry points matter here
            let called_by_shim = shim.matches(h.as_str()).count() > 1;
            let dynamic = DYNAMIC_PREFIXES.iter().any(|p| h.starts_with(p));
            let alive = used_in_rows.contains(&h) || used_in_rust.contains(&h) || called_by_shim || dynamic || !is_helper_name(&h, target);
            let listed = allowed.contains(&h.as_str());
            if !alive && !listed {
                bad.push(format!("{target:?}: helper `{h}` is defined in the shim and used by no row, no emitter and no other helper: delete it"));
            }
            if alive && listed {
                bad.push(format!("{target:?}: `{h}` is on the dead-helper list but is used: remove it from the list"));
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

fn is_helper_name(name: &str, target: Target) -> bool {
    match target {
        Target::Js => name.starts_with('$'),
        Target::Go => name.starts_with("wj"),
    }
}

// ---------------------------------------------------------------------------------------------
// G6: the vocabulary cap
// ---------------------------------------------------------------------------------------------

/// The variants of `pub enum <name>` in `src/stdlib_surface/lowering.rs`, counted from the source text (so adding a
/// variant, even one with data, is seen without compiling it).
fn variants(source: &str, name: &str) -> Vec<String> {
    let head = format!("pub enum {name} {{");
    let start = source.find(&head).unwrap_or_else(|| panic!("`{head}` not found in lowering.rs"));
    let mut out = Vec::new();
    let mut depth = 0i32;
    for line in source[start + head.len()..].lines().skip(1).chain(std::iter::once("")) {
        let t = line.trim();
        if depth == 0 && t == "}" {
            break;
        }
        if depth == 0 && t.starts_with(|c: char| c.is_ascii_uppercase()) {
            let ident: String = t.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            out.push(ident);
        }
        depth += t.matches('{').count() as i32 - t.matches('}').count() as i32;
    }
    out
}

/// A new variant of one of these needs this test edited with the reason: the vocabulary names type shapes and
/// emitter capabilities, never std methods (section 2.3 of the design).
const VOCABULARY: &[(&str, usize, &str)] = &[
    ("Guard", 5, "elem_prim, elem_prim_not_bool: the shape of the receiver's element (Vec::contains, sort); t_wide, t_i32, t_u32: the receiver's own integer type (a 64-bit BigInt receiver; `wrapping_mul` on the 32-bit types is `Math.imul`). `f32` joins when a float row needs it"),
    ("Hook", 1, "display: the type-directed `to_string` emitter; `clone` joins with the Vec stage"),
    ("Modifier", 1, "f64: Go's operand widening for Float<T>; `str` and `num` join with the String / Integer stages"),
    ("Fact", 11, "bits signed wide min max zero one chk go: the numeric helpers' facts of the Integer rows; wrap_open, wrap_close: the text that brackets an expression wrapped into T's range (asIntN(64, ..), `(.. | 0)`, `((.. << 24) >> 24)`: a template is linear, so the bracket is two facts)"),
    ("Rule", 3, "Emit, Hook, Unsupported; `Assign` joins with the String stage"),
    ("Target", 2, "Js, Go; TypeScript uses the JavaScript rules; `Rust` joins when the Rust backend reads the table"),
    ("CoreType", 2, "one per migrated std type (a stage adds its types): Float, Integer"),
    ("Widen", 1, "F64"),
];

#[test]
fn the_vocabulary_has_the_pinned_number_of_variants() {
    let source = read("src/stdlib_surface/lowering.rs");
    let mut bad = Vec::new();
    for (name, want, why) in VOCABULARY {
        let got = variants(&source, name);
        if got.len() != *want {
            bad.push(format!("`{name}` has {} variants {got:?}, the gate pins {want} ({why}). Edit VOCABULARY with the reason for the new one.", got.len()));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn the_vocabulary_lists_are_complete() {
    assert_eq!(Guard::ALL.len(), variants(&read("src/stdlib_surface/lowering.rs"), "Guard").len());
    assert_eq!(Hook::ALL.len(), variants(&read("src/stdlib_surface/lowering.rs"), "Hook").len());
    assert_eq!(Modifier::ALL.len(), variants(&read("src/stdlib_surface/lowering.rs"), "Modifier").len());
    assert_eq!(Fact::ALL.len(), variants(&read("src/stdlib_surface/lowering.rs"), "Fact").len());
    assert_eq!(CoreType::ALL.len(), variants(&read("src/stdlib_surface/lowering.rs"), "CoreType").len());
    assert_eq!(Target::ALL.len(), variants(&read("src/stdlib_surface/lowering.rs"), "Target").len());
}

// ---------------------------------------------------------------------------------------------
// G8: ADR-0039 cross-check
// ---------------------------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct Removed {
    id: String,
    pattern: String,
}

#[derive(serde::Deserialize)]
struct StdSurfaceToml {
    removed: Vec<Removed>,
}

/// Declarations of `std/rust_core.wji` that spell a removed form and are not `@internal`. These exist because the
/// declaration is the Rust core method the surface lowers to (checked against rustc), not a spelling a program may
/// write; each goes when its row is `@internal(reason = "...")` or the removed form is no longer a Rust method name.
/// The list may only shrink; a NEW declaration named like a removed form fails. (Stage 1 measured it: the Float rows
/// that spell a removed form carry `@internal` instead.)
const REMOVED_SPELLED_WITHOUT_INTERNAL: &[&str] = &[
    "BTreeMap::contains_key",
    "HashMap::capacity",
    "HashMap::contains_key",
    "HashSet::capacity",
    "Iterator::cloned",
    "Iterator::copied",
    "Iterator::for_each",
    "Iterator::nth",
    "Option::cloned",
    "Option::copied",
    "Option::unwrap",
    "Option::unwrap_or",
    "Option::unwrap_or_default",
    "Result::expect_err",
    "Result::unwrap",
    "Result::unwrap_err",
    "Result::unwrap_or",
    "Result::unwrap_or_default",
    "String::capacity",
    "String::push_str",
    "Vec::capacity",
    "Vec::extend_from_slice",
    "Vec::into_iter",
    "Vec::iter",
    "Vec::iter_mut",
    "VecDeque::back",
    "VecDeque::front",
    "VecDeque::pop_back",
    "VecDeque::push_back",
];

/// The `removed` rows whose pattern the `regex` crate can run. A pattern with look-around (`(?<![.\w])max\(`) matches a
/// free function call, never the `x.name(` spelling of a method declaration, so it is skipped here; the count of
/// skipped rows is pinned so a new one is noticed.
const REMOVED_PATTERNS_WITH_LOOKAROUND: usize = 1;
// (A row with an empty pattern, R094, is a documentation row the inventory skips: it matches nothing here either.)

fn removed_forms() -> Vec<(String, regex::Regex)> {
    let parsed: StdSurfaceToml = toml::from_str(&read("docs/std_surface.toml")).expect("docs/std_surface.toml parses");
    let (lookaround, plain): (Vec<_>, Vec<_>) = parsed.removed.into_iter().filter(|r| !r.pattern.is_empty()).partition(|r| r.pattern.contains("(?<") || r.pattern.contains("(?!") || r.pattern.contains("(?="));
    assert_eq!(lookaround.len(), REMOVED_PATTERNS_WITH_LOOKAROUND, "a `removed` row with look-around: {:?}", lookaround.iter().map(|r| &r.id).collect::<Vec<_>>());
    plain.into_iter().map(|r| { let re = regex::Regex::new(&r.pattern).unwrap_or_else(|e| panic!("removed row {}: {e}", r.id)); (r.id, re) }).collect()
}

/// `Trait::method` of every declaration whose call spelling a removed row matches and that lacks `@internal`.
fn removed_spellings(core: &CoreSurface) -> BTreeSet<String> {
    let removed = removed_forms();
    let mut out = BTreeSet::new();
    for t in core.declared_traits() {
        for m in &t.methods {
            let samples = [format!("x.{}(", m.name), format!("x.{}()", m.name)];
            let internal = m.decorators.iter().any(|d| d.name == "internal");
            // the pattern must spell the method call itself (`\.to_owned\(`: the match includes the dot at index 1), not a
            // free function of the same name (`\blen\(` matches after the dot)
            if !internal && removed.iter().any(|(_, re)| samples.iter().any(|sample| re.find(sample).is_some_and(|m| m.start() <= 1))) {
                out.insert(format!("{}::{}", t.name, m.name));
            }
        }
    }
    out
}

#[test]
fn a_declaration_named_like_a_removed_form_is_internal() {
    let found = removed_spellings(&surface());
    let allowed: BTreeSet<String> = REMOVED_SPELLED_WITHOUT_INTERNAL.iter().map(|s| s.to_string()).collect();
    let new: Vec<&String> = found.difference(&allowed).collect();
    let stale: Vec<&String> = allowed.difference(&found).collect();
    assert!(new.is_empty(), "declared with the spelling of a `removed` row of docs/std_surface.toml (ADR-0039); mark it `@internal(reason = \"...\")` naming the desugaring that produces it: {new:?}");
    assert!(stale.is_empty(), "REMOVED_SPELLED_WITHOUT_INTERNAL lists declarations that are fine now: {stale:?}");
}

#[test]
fn the_removed_spelling_check_sees_a_planted_declaration() {
    let text = read("std/rust_core.wji");
    let at = text.find("pub trait String {").expect("trait String");
    let planted = format!("{}pub trait String {{\n    fn to_owned(self) -> String;\n{}", &text[..at], &text[at + "pub trait String {\n".len()..]);
    let found = removed_spellings(&CoreSurface::parse_source(&planted));
    assert!(found.contains("String::to_owned"), "a `to_owned` declaration without @internal must be seen: {found:?}");
}

/// The canonical rows of docs/std_surface.toml, per migrated type (`type = "float"`), that do not resolve to exactly one
/// declaration of the type's trait: the row's `lower` spelling or its name. Shrink-only; each stage measured its own.
/// Integer: `cmp` needs `Ordering`; the byte conversions are the width traits'; the byte (`u8`) predicates and case
/// conversions are the char stage's.
const ROWS_WITHOUT_ONE_DECLARATION: &[(CoreType, &str, &[&str])] = &[
    (CoreType::Float, "float", &["cmp", "log", "pow", "rem_euclid", "copysign", "to_bits", "to_be_bytes", "to_le_bytes"]),
    (CoreType::Integer, "int", &["cmp", "to_be_bytes", "to_le_bytes", "is_digit", "is_alphabetic", "is_whitespace", "to_uppercase", "to_lowercase"]),
];

#[derive(serde::Deserialize)]
struct MethodRow {
    #[serde(rename = "type")]
    ty: String,
    name: String,
    #[serde(default)]
    lower: Option<String>,
}

#[derive(serde::Deserialize)]
struct MethodsToml {
    methods: Vec<MethodRow>,
}

#[test]
fn every_canonical_row_resolves_to_one_declaration() {
    let parsed: MethodsToml = toml::from_str(&read("docs/std_surface.toml")).expect("docs/std_surface.toml parses");
    let core = surface();
    let mut bad = Vec::new();
    for ty in MIGRATED {
        let (_, toml_type, allowed) = ROWS_WITHOUT_ONE_DECLARATION.iter().find(|(t, _, _)| t == ty).unwrap_or_else(|| panic!("{}: no entry in ROWS_WITHOUT_ONE_DECLARATION", ty.trait_name()));
        let declared: BTreeSet<String> = declared(&core, *ty).into_iter().collect();
        let allowed: BTreeSet<&str> = allowed.iter().copied().collect();
        let mut unresolved = BTreeSet::new();
        for r in parsed.methods.iter().filter(|r| r.ty == *toml_type) {
            let target = r.lower.as_deref().filter(|l| l.chars().all(|c| c.is_alphanumeric() || c == '_')).unwrap_or(&r.name);
            if !declared.contains(target) {
                unresolved.insert(r.name.as_str());
            }
        }
        for name in &unresolved {
            if !allowed.contains(name) {
                bad.push(format!("{toml_type} row `{name}` of docs/std_surface.toml resolves to no declaration of `trait {}`", ty.trait_name()));
            }
        }
        for name in &allowed {
            if !unresolved.contains(name) {
                bad.push(format!("`{name}` resolves now: delete it from ROWS_WITHOUT_ONE_DECLARATION ({})", ty.trait_name()));
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
