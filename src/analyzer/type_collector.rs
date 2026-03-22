//! Collect type names referenced in a Windjammer program for automatic Rust `use` generation.
//!
//! Walks the AST type positions (fields, function signatures, impl methods, type aliases)
//! and extracts names that likely need `use super::...` in generated submodule `.rs` files.

use crate::parser::ast::core::{EnumVariantData, Item};
use crate::parser::ast::types::AssociatedType;
use crate::parser::{FunctionDecl, ImplBlock, Program, StructDecl, TraitDecl, Type, TypeParam};

use std::collections::HashSet;

/// Standard library / runtime generic containers — we recurse into type args but never import the base.
fn is_stdlib_container(base: &str) -> bool {
    matches!(
        base,
        "Vec"
            | "Option"
            | "Result"
            | "HashMap"
            | "HashSet"
            | "BTreeMap"
            | "BTreeSet"
            | "Box"
            | "Arc"
            | "Rc"
            | "RefCell"
            | "Cell"
            | "Mutex"
            | "RwLock"
            | "Weak"
            | "Pin"
            | "PhantomData"
            | "NonNull"
            | "VecDeque"
            | "BinaryHeap"
            | "LinkedList"
            | "SmallVec"
            | "Cow"
            | "Iter"
            | "Slice"
            | "Signal"
    )
}

/// Types handled without a `use` (prelude, primitives, or synthesized paths in codegen).
fn skip_standalone_custom_name(name: &str) -> bool {
    matches!(
        name,
        "str" | "string" | "Self" | "self" | "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8"
            | "u16" | "u32" | "u64" | "u128" | "usize" | "f32" | "f64" | "bool" | "char" | "()"
    )
}

/// Push a custom path for import (may be `Type` or `module::Type`).
fn push_custom_path(out: &mut HashSet<String>, name: &str) {
    if skip_standalone_custom_name(name) {
        return;
    }
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return;
    }
    // Only types / modules following Rust conventions (uppercase start or qualified path).
    if trimmed.contains('.') {
        out.insert(trimmed.to_string());
    } else if trimmed
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase())
    {
        out.insert(trimmed.to_string());
    }
}

/// Recursively collect referenced type paths from a [`Type`].
pub fn collect_type_references(ty: &Type, out: &mut HashSet<String>) {
    match ty {
        Type::Custom(name) => push_custom_path(out, name),
        Type::Parameterized(base, args) => {
            if !is_stdlib_container(base.as_str()) {
                push_custom_path(out, base);
            }
            for a in args {
                collect_type_references(a, out);
            }
        }
        Type::Option(inner) | Type::Vec(inner) => collect_type_references(inner, out),
        Type::Result(ok, err) => {
            collect_type_references(ok, out);
            collect_type_references(err, out);
        }
        Type::Array(inner, _) => collect_type_references(inner, out),
        Type::Reference(inner) | Type::MutableReference(inner) => {
            collect_type_references(inner, out);
        }
        Type::RawPointer { pointee, .. } => collect_type_references(pointee, out),
        Type::Tuple(items) => {
            for t in items {
                collect_type_references(t, out);
            }
        }
        Type::FunctionPointer {
            params,
            return_type,
        } => {
            for p in params {
                collect_type_references(p, out);
            }
            if let Some(rt) = return_type {
                collect_type_references(rt, out);
            }
        }
        Type::TraitObject(_) => {}
        Type::ImplTrait(_) => {}
        Type::Associated(_, _) => {}
        Type::Generic(_) | Type::Infer => {}
        Type::Int
        | Type::Int32
        | Type::Uint
        | Type::Float
        | Type::Bool
        | Type::String => {}
    }
}

fn collect_from_type_params(_params: &[TypeParam], _out: &mut HashSet<String>) {
    // Intentionally ignore trait bounds (`T: Display`): those are language traits / prelude,
    // not sibling modules to import via `use super::...`.
}

fn collect_from_associated_types(assoc: &[AssociatedType], out: &mut HashSet<String>) {
    for a in assoc {
        if let Some(t) = &a.concrete_type {
            collect_type_references(t, out);
        }
    }
}

fn collect_from_function_decl<'ast>(decl: &FunctionDecl<'ast>, out: &mut HashSet<String>) {
    for p in &decl.parameters {
        collect_type_references(&p.type_, out);
    }
    if let Some(rt) = &decl.return_type {
        collect_type_references(rt, out);
    }
}

fn collect_from_struct_decl<'ast>(decl: &StructDecl<'ast>, out: &mut HashSet<String>) {
    collect_from_type_params(&decl.type_params, out);
    for f in &decl.fields {
        collect_type_references(&f.field_type, out);
    }
}

fn collect_from_impl_block<'ast>(block: &ImplBlock<'ast>, out: &mut HashSet<String>) {
    collect_from_type_params(&block.type_params, out);
    if let Some(args) = &block.trait_type_args {
        for t in args {
            collect_type_references(t, out);
        }
    }
    collect_from_associated_types(&block.associated_types, out);
    for f in &block.functions {
        collect_from_function_decl(f, out);
    }
}

fn collect_from_trait_decl<'ast>(decl: &TraitDecl<'ast>, out: &mut HashSet<String>) {
    collect_from_associated_types(&decl.associated_types, out);
    for m in &decl.methods {
        for p in &m.parameters {
            collect_type_references(&p.type_, out);
        }
        if let Some(rt) = &m.return_type {
            collect_type_references(rt, out);
        }
        if let Some(body) = &m.body {
            // Default method bodies can mention types only in expressions — skipped for imports.
            let _ = body;
        }
    }
}

/// Struct, enum, trait, and type-alias names defined in this compilation unit.
pub fn collect_local_type_names<'ast>(program: &Program<'ast>) -> HashSet<String> {
    let mut local = HashSet::new();
    for item in &program.items {
        match item {
            Item::Struct { decl, .. } => {
                local.insert(decl.name.clone());
            }
            Item::Enum { decl, .. } => {
                local.insert(decl.name.clone());
            }
            Item::Trait { decl, .. } => {
                local.insert(decl.name.clone());
            }
            Item::TypeAlias { name, .. } => {
                local.insert(name.clone());
            }
            _ => {}
        }
    }
    local
}

/// Every type path referenced from type positions in the program (before filtering).
pub fn collect_referenced_type_paths<'ast>(program: &Program<'ast>) -> HashSet<String> {
    let mut out = HashSet::new();
    for item in &program.items {
        match item {
            Item::Struct { decl, .. } => collect_from_struct_decl(decl, &mut out),
            Item::Enum { decl, .. } => {
                collect_from_type_params(&decl.type_params, &mut out);
                for v in &decl.variants {
                    match &v.data {
                        EnumVariantData::Unit => {}
                        EnumVariantData::Tuple(types) => {
                            for t in types {
                                collect_type_references(t, &mut out);
                            }
                        }
                        EnumVariantData::Struct(fields) => {
                            for (_, t) in fields {
                                collect_type_references(t, &mut out);
                            }
                        }
                    }
                }
            }
            Item::Trait { decl, .. } => collect_from_trait_decl(decl, &mut out),
            Item::Impl { block, .. } => collect_from_impl_block(block, &mut out),
            Item::Function { decl, .. } => collect_from_function_decl(decl, &mut out),
            Item::Const { type_, .. } | Item::Static { type_, .. } | Item::ExternLet { type_, .. } => {
                collect_type_references(type_, &mut out);
            }
            Item::TypeAlias { target, .. } => collect_type_references(target, &mut out),
            Item::Mod { items, .. } => {
                let nested = Program { items: items.clone() };
                out.extend(collect_referenced_type_paths(&nested));
            }
            Item::Use { .. } | Item::BoundAlias { .. } => {}
        }
    }
    out
}

/// Type paths that should become `use super::...` lines (not defined locally).
pub fn external_type_import_paths<'ast>(program: &Program<'ast>) -> Vec<String> {
    let local = collect_local_type_names(program);
    let referenced = collect_referenced_type_paths(program);
    let mut external: Vec<String> = referenced
        .into_iter()
        .filter(|path| {
            if path.contains('.') {
                true
            } else {
                !local.contains(path)
            }
        })
        .collect();
    external.sort();
    external
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn external_for_snippet(src: &str) -> Vec<String> {
        let owned = src.to_string();
        let mut lexer = Lexer::new(&owned);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new_with_source(tokens, "test.wj".to_string(), owned);
        let program = parser.parse().expect("parse");
        external_type_import_paths(&program)
    }

    fn local_for_snippet(src: &str) -> std::collections::HashSet<String> {
        let owned = src.to_string();
        let mut lexer = Lexer::new(&owned);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new_with_source(tokens, "test.wj".to_string(), owned);
        let program = parser.parse().expect("parse");
        collect_local_type_names(&program)
    }

    #[test]
    fn collect_struct_field_custom_type() {
        let local = local_for_snippet(
            r#"
pub struct DialogueChoice {
    pub id: u32,
    pub next_line: DialogueLineId
}
"#,
        );
        assert!(local.contains("DialogueChoice"));
        let ext = external_for_snippet(
            r#"
pub struct DialogueChoice {
    pub id: u32,
    pub next_line: DialogueLineId
}
"#,
        );
        assert!(ext.contains(&"DialogueLineId".to_string()));
    }

    #[test]
    fn collect_hashmap_value_types_only() {
        let ext = external_for_snippet(
            r#"
pub struct Manager {
    achievements: HashMap<AchievementId, Achievement>,
    users: Vec<User>,
    current_quest: Option<Quest>,
    count: i32,
    name: String,
    active: bool
}
"#,
        );
        assert!(ext.contains(&"AchievementId".to_string()));
        assert!(ext.contains(&"Achievement".to_string()));
        assert!(ext.contains(&"User".to_string()));
        assert!(ext.contains(&"Quest".to_string()));
        assert!(!ext.contains(&"HashMap".to_string()));
        assert!(!ext.contains(&"Vec".to_string()));
        assert!(!ext.contains(&"Option".to_string()));
    }

    #[test]
    fn collect_impl_method_param_type() {
        let ext = external_for_snippet(
            r#"
pub struct Manager {
    items: Vec<Item>
}

impl Manager {
    pub fn add_item(self, item: Item) { }
}
"#,
        );
        assert!(ext.contains(&"Item".to_string()));
    }

    #[test]
    fn local_type_not_external() {
        let ext = external_for_snippet(
            r#"
pub struct Inner { x: i32 }
pub struct Outer { inner: Inner }
"#,
        );
        assert!(!ext.contains(&"Inner".to_string()));
    }
}
