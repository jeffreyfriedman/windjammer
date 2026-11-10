//! Integer widths the compiled build would mix (rustc E0308, reported as WJ0003), found by `wj check` and
//! therefore before an interpreted run (ADR-0033, family A of `docs/drift/findings.md`).
//!
//! The interpreter has one integer type. The compiled backend sometimes types a literal (or the binding it
//! initialises) `i32` next to an `int` (`i64`) value, and rustc then rejects the program, so the program runs
//! interpreted and fails only when built. Like the move prediction (ADR-0030) this reads generated Rust, so it
//! follows the compiler's own width choices; `cli::widths_gate` runs it on the checker's in-process text as a
//! trigger and reports only what it finds in the Rust `wj build` emits (the two differ in widths). It reports only places where both widths are certain from the generated text (suffixed literals, casts,
//! annotated bindings, parameters, fields, the file's own functions and methods, `Option` payloads) and
//! differ where Rust requires one type. Everything uncertain is left to the compiled build.

mod types;
mod walk;

use crate::diagnostics::{Diagnostic, Span};
use std::collections::HashMap;
use std::path::Path;
pub use types::Ty;
pub use walk::Clash;
use types::Items;
use walk::{Sig, Walker};

/// The Rust `wj check` reads for the single file at `path` (in-process generation; tests and tools).
pub fn generated_rust(path: &Path) -> Option<String> {
    let source = std::fs::read_to_string(path).ok()?;
    let file = path.to_string_lossy().to_string();
    let tokens = crate::lexer::Lexer::new(&source).tokenize_with_locations();
    let program = crate::parser::Parser::new_with_source(tokens, file, source.clone()).parse().ok()?;
    super::moves::generate(&program)
}

/// WJ0003 diagnostics for one file, from its generated Rust.
pub fn predict(rust: &str, source: &str, file: &str) -> Vec<Diagnostic> {
    clashes(rust).iter().map(|c| diagnostic(c, source, file)).collect()
}

/// Every place in a generated Rust file where two different integer widths must be one type.
pub fn clashes(rust: &str) -> Vec<Clash> {
    let Ok(f) = syn::parse_file(rust) else { return Vec::new() };
    let items = collect_items(&f);
    let (fns, methods) = signatures(&f, &items);
    let mut w = Walker::new(&items, &fns, &methods);
    for item in &f.items {
        match item {
            syn::Item::Fn(func) => {
                w.self_ty = None;
                w.function(&func.sig, &func.block);
            }
            syn::Item::Impl(imp) => {
                w.self_ty = impl_self(imp);
                for ii in &imp.items {
                    if let syn::ImplItem::Fn(m) = ii {
                        w.function(&m.sig, &m.block);
                    }
                }
            }
            _ => {}
        }
    }
    w.found
}

/// The name of a non-generic `impl` target declared in the file.
fn impl_self(imp: &syn::ItemImpl) -> Option<String> {
    let syn::Type::Path(p) = &*imp.self_ty else { return None };
    let plain = p.qself.is_none() && p.path.segments.len() == 1 && matches!(p.path.segments[0].arguments, syn::PathArguments::None);
    (plain && imp.generics.params.is_empty()).then(|| p.path.segments[0].ident.to_string())
}

/// Item names of the whole file (nested ones too). A struct or function whose name occurs twice (an item
/// inside a function or a module may shadow it) is not read at all.
fn collect_items(f: &syn::File) -> Items {
    #[derive(Default)]
    struct Names(HashMap<String, usize>);
    impl<'ast> syn::visit::Visit<'ast> for Names {
        fn visit_item(&mut self, i: &'ast syn::Item) {
            let name = match i {
                syn::Item::Fn(x) => Some(&x.sig.ident),
                syn::Item::Struct(x) => Some(&x.ident),
                syn::Item::Enum(x) => {
                    for v in &x.variants {
                        *self.0.entry(v.ident.to_string()).or_default() += 1;
                    }
                    Some(&x.ident)
                }
                syn::Item::Const(x) => Some(&x.ident),
                syn::Item::Static(x) => Some(&x.ident),
                syn::Item::Trait(x) => Some(&x.ident),
                syn::Item::Type(x) => Some(&x.ident),
                syn::Item::Union(x) => Some(&x.ident),
                syn::Item::Mod(x) => Some(&x.ident),
                _ => None,
            };
            if let Some(n) = name {
                *self.0.entry(n.to_string()).or_default() += 1;
            }
            syn::visit::visit_item(self, i);
        }
        fn visit_use_rename(&mut self, r: &'ast syn::UseRename) {
            *self.0.entry(r.rename.to_string()).or_default() += 2;
        }
    }
    let mut names = Names::default();
    syn::visit::Visit::visit_file(&mut names, f);
    let once = names.0.iter().filter(|(_, c)| **c == 1).map(|(n, _)| n.clone()).collect();
    let mut items = Items { defined: names.0.keys().cloned().collect(), once, structs: HashMap::new() };
    let unique = |n: &str| names.0.get(n) == Some(&1);
    let declared: Vec<&syn::ItemStruct> = f
        .items
        .iter()
        .filter_map(|i| match i {
            syn::Item::Struct(s) if s.generics.params.is_empty() && unique(&s.ident.to_string()) => Some(s),
            _ => None,
        })
        .collect();
    // names first, so a field may name another struct of the file
    for s in &declared {
        items.structs.insert(s.ident.to_string(), HashMap::new());
    }
    for s in &declared {
        let fields: HashMap<String, Ty> = s
            .fields
            .iter()
            .filter_map(|fl| fl.ident.as_ref().map(|i| (i.to_string(), items.of(&fl.ty, Some(s.ident.to_string().as_str())))))
            .collect();
        items.structs.insert(s.ident.to_string(), fields);
    }
    items
}

fn sig_of(items: &Items, sig: &syn::Signature, self_ty: Option<&str>) -> Option<Sig> {
    if !sig.generics.params.is_empty() {
        return None;
    }
    let params = sig
        .inputs
        .iter()
        .filter_map(|a| match a {
            syn::FnArg::Typed(p) => Some(items.of(&p.ty, self_ty)),
            syn::FnArg::Receiver(_) => None,
        })
        .collect();
    let ret = match &sig.output {
        syn::ReturnType::Type(_, t) => items.of(t, self_ty),
        syn::ReturnType::Default => Ty::Unknown,
    };
    Some(Sig { params, ret })
}

type Methods = HashMap<(String, String), Sig>;

/// Top-level functions and inherent methods (not trait impls) of the file, by name.
fn signatures(f: &syn::File, items: &Items) -> (HashMap<String, Sig>, Methods) {
    let mut fns = HashMap::new();
    let mut methods = HashMap::new();
    for item in &f.items {
        match item {
            syn::Item::Fn(func) => {
                if let Some(s) = sig_of(items, &func.sig, None) {
                    fns.insert(func.sig.ident.to_string(), s);
                }
            }
            syn::Item::Impl(imp) if imp.trait_.is_none() => {
                let Some(t) = impl_self(imp).filter(|t| items.structs.contains_key(t)) else { continue };
                for ii in &imp.items {
                    if let syn::ImplItem::Fn(m) = ii {
                        if let Some(s) = sig_of(items, &m.sig, Some(t.as_str())) {
                            methods.insert((t.clone(), m.sig.ident.to_string()), s);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    // a function name defined more than once anywhere in the file (nested, in a module, as a variant) is not read
    fns.retain(|n, _| items.once.contains(n));
    (fns, methods)
}

fn diagnostic(c: &Clash, source: &str, file: &str) -> Diagnostic {
    let (a, b) = (c.left.show(), c.right.show());
    let mut d = Diagnostic::error(
        "WJ0003",
        format!("the compiled build would reject this: `{}` needs one integer type but gets `{a}` and `{b}` (in `fn {}` of the generated code)", c.expr, c.function),
    )
    .with_help("give the literal or the binding it comes from the type `int` (`let five: int = 5`, `1 as int`); the interpreter runs the program, the compiled build does not")
    .with_note("`int` is `i64`; the compiled backend typed one side of this expression `i32` (or another width), and Rust never converts integer widths implicitly. The backend defect is tracked as family A in docs/drift/findings.md");
    if let Some(sp) = locate(c, source, file) {
        d = d.at(sp.with_label("widths differ here"));
    }
    d
}

/// The first use of the clash's variable inside the source function of the same name (or the function).
fn locate(c: &Clash, source: &str, file: &str) -> Option<Span> {
    let header = format!("fn {}", c.function);
    let fn_at = source.match_indices(&header).map(|(i, _)| i).find(|i| {
        let after = source[i + header.len()..].chars().next();
        after.is_some_and(|ch| ch == '(' || ch == '<' || ch.is_whitespace())
    })?;
    let body_from = fn_at + header.len();
    let var = c.var.as_deref().map(|v| v.trim_start_matches('_')).filter(|v| !v.is_empty());
    let bytes = source.as_bytes();
    // the token after the variable in the generated expression (`item +`), to prefer the use over the binding
    let next = c.var.as_deref().and_then(|v| {
        let toks: Vec<&str> = c.expr.split_whitespace().collect();
        toks.iter().position(|t| *t == v).and_then(|i| toks.get(i + 1)).map(|t| t.to_string())
    });
    let uses = |v: &str| {
        source[body_from..].match_indices(v).map(|(i, _)| body_from + i).filter(|&i| {
            let before = i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
            let end = i + v.len();
            let after = end >= bytes.len() || !(bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_');
            before && after
        }).collect::<Vec<usize>>()
    };
    let hit = var.and_then(|v| {
        let all = uses(v);
        let followed = |i: &&usize| next.as_deref().is_some_and(|n| source[**i + v.len()..].trim_start_matches([' ', '\t']).starts_with(n));
        all.iter().find(followed).or(all.first()).copied()
    });
    let (start, len) = match (hit, var) {
        (Some(i), Some(v)) => (i, v.len()),
        _ => (fn_at, header.len()),
    };
    let line = source[..start].matches('\n').count() + 1;
    let line_start = source[..start].rfind('\n').map_or(0, |n| n + 1);
    let col = source[line_start..start].chars().count() + 1;
    let text = &source[start..start + len];
    Some(Span {
        file: file.to_string(),
        byte_start: start,
        byte_end: start + len,
        line_start: line,
        col_start: col,
        line_end: line,
        col_end: col + text.chars().count(),
        label: None,
    })
}
