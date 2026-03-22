//! Collect type names referenced in a Windjammer program for automatic Rust `use` generation.
//!
//! Walks the AST type positions (fields, function signatures, impl methods, type aliases)
//! and extracts names that likely need `use super::...` in generated submodule `.rs` files.

use crate::parser::ast::core::{EnumVariantData, Item};
use crate::parser::ast::types::AssociatedType;
use crate::parser::{FunctionDecl, ImplBlock, Program, StructDecl, TraitDecl, Type, TypeParam};

use std::collections::HashSet;
use std::path::{Component, Path};

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
        "str"
            | "string"
            | "String"
            | "Self"
            | "self"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
            | "bool"
            | "char"
            | "()"
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
        Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool | Type::String => {}
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
            Item::Const { type_, .. }
            | Item::Static { type_, .. }
            | Item::ExternLet { type_, .. } => {
                collect_type_references(type_, &mut out);
            }
            Item::TypeAlias { target, .. } => collect_type_references(target, &mut out),
            Item::Mod { items, .. } => {
                let nested = Program {
                    items: items.clone(),
                };
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

/// `use super::*` imports every sibling from the parent module; emitting additional
/// `use super::Type` lines duplicates those names and triggers Rust E0252.
pub fn has_super_star_glob_import<'ast>(program: &Program<'ast>) -> bool {
    program.items.iter().any(|item| {
        matches!(
            item,
            Item::Use { path, .. }
                if path.len() == 2 && path[0] == "super" && path[1] == "*"
        )
    })
}

/// Simple type names already brought in by explicit `use` items (last path segment, uppercase).
/// Globs (`::*`) are ignored because we cannot know which symbols they provide.
fn explicitly_imported_unqualified_type_names<'ast>(program: &Program<'ast>) -> HashSet<String> {
    let mut names = HashSet::new();
    for item in &program.items {
        let Item::Use { path, .. } = item else {
            continue;
        };
        if path.len() == 2 && path[0] == "super" && path[1] == "*" {
            continue;
        }
        let Some(last) = path.last() else {
            continue;
        };
        if last == "*" || last.contains("::{") {
            continue;
        }
        if last.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
            names.insert(last.clone());
        }
    }
    names
}

/// Paths for generated `use super::...` / `use super::super::...` lines in module `.rs` output.
///
/// Skips generation when `use super::*` is present (glob already covers siblings). Drops
/// unqualified names that are already imported explicitly (e.g. `use crate::math::Vec3`).
pub fn auto_super_type_import_paths<'ast>(program: &Program<'ast>) -> Vec<String> {
    if has_super_star_glob_import(program) {
        return Vec::new();
    }
    let explicit = explicitly_imported_unqualified_type_names(program);
    let mut paths: Vec<String> = external_type_import_paths(program)
        .into_iter()
        .filter(|p| {
            if !p.contains('.') {
                !explicit.contains(p)
            } else {
                true
            }
        })
        .collect();
    paths.sort();
    paths
}

/// Rust module path segments for a `.wj` file relative to the library source root (multipass).
///
/// Example: `achievement/manager.wj` → `["achievement", "manager"]`; `achievement/mod.wj` → `["achievement"]`.
pub fn wj_file_to_module_path(base: &Path, wj: &Path) -> Option<Vec<String>> {
    let rel = wj.strip_prefix(base).ok()?;
    let mut segs: Vec<String> = rel
        .parent()
        .map(|p| {
            p.components()
                .filter_map(|c| match c {
                    Component::Normal(x) => x.to_str().map(String::from),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    let stem = rel.file_stem()?.to_str()?;
    if stem != "mod" {
        segs.push(stem.to_string());
    }
    if segs.is_empty() {
        return None;
    }
    Some(segs)
}

fn longest_common_prefix_len(a: &[String], b: &[String]) -> usize {
    a.iter()
        .zip(b.iter())
        .take_while(|(x, y)| x == y)
        .count()
}

/// Build a `use ...::TypeName;` path (including leading `super::` segments) from the current
/// module to the module that defines `type_name`.
///
/// `current_module` / `defining_module` are full segment paths from the crate’s `src` root, e.g.
/// `["achievement", "manager"]` and `["achievement", "achievement_id"]`.
pub fn rust_use_path_from_module_to_type(
    current_module: &[String],
    defining_module: &[String],
    type_name: &str,
) -> Option<String> {
    if defining_module.is_empty() || type_name.is_empty() {
        return None;
    }
    let lcp = longest_common_prefix_len(current_module, defining_module);
    let ups = current_module.len().saturating_sub(lcp);
    let down = &defining_module[lcp..];
    let mut parts: Vec<&str> = Vec::new();
    for _ in 0..ups {
        parts.push("super");
    }
    for seg in down {
        parts.push(seg.as_str());
    }
    parts.push(type_name);
    Some(parts.join("::"))
}

/// Split a dotted reference like `state_machine.Transition` into module hint + type name.
pub fn split_qualified_type_path(path: &str) -> (&str, &str) {
    if let Some((prefix, last)) = path.rsplit_once('.') {
        if !prefix.is_empty() && !last.is_empty() {
            return (prefix, last);
        }
    }
    ("", path)
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

    #[test]
    fn auto_super_paths_empty_when_super_glob_present() {
        let owned = r#"
use super::*

pub struct UserManager {
    pub user: User
}
"#
        .to_string();
        let mut lexer = Lexer::new(&owned);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new_with_source(tokens, "test.wj".to_string(), owned);
        let program = parser.parse().expect("parse");
        assert!(has_super_star_glob_import(&program));
        assert!(auto_super_type_import_paths(&program).is_empty());
    }

    #[test]
    fn auto_super_paths_omit_explicitly_imported_unqualified_types() {
        let owned = r#"
use crate::math::Vec3

pub struct Manager {
    pub position: Vec3,
    pub user: User
}
"#
        .to_string();
        let mut lexer = Lexer::new(&owned);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new_with_source(tokens, "test.wj".to_string(), owned);
        let program = parser.parse().expect("parse");
        assert!(!has_super_star_glob_import(&program));
        let paths = auto_super_type_import_paths(&program);
        assert!(
            paths.contains(&"User".to_string()),
            "expected User from sibling; got {:?}",
            paths
        );
        assert!(
            !paths.contains(&"Vec3".to_string()),
            "Vec3 is already imported; got {:?}",
            paths
        );
    }

    #[test]
    fn rust_use_path_sibling_submodule() {
        let cur = vec!["achievement".into(), "manager".into()];
        let def = vec!["achievement".into(), "achievement_id".into()];
        assert_eq!(
            rust_use_path_from_module_to_type(&cur, &def, "AchievementId").as_deref(),
            Some("super::achievement_id::AchievementId")
        );
    }

    #[test]
    fn rust_use_path_flat_sibling() {
        let cur = vec!["manager".into()];
        let def = vec!["user".into()];
        assert_eq!(
            rust_use_path_from_module_to_type(&cur, &def, "User").as_deref(),
            Some("super::user::User")
        );
    }

    #[test]
    fn rust_use_path_cousin_modules() {
        let cur = vec!["ai".into(), "foo".into(), "manager".into()];
        let def = vec!["ai".into(), "state_machine".into()];
        assert_eq!(
            rust_use_path_from_module_to_type(&cur, &def, "Transition").as_deref(),
            Some("super::super::state_machine::Transition")
        );
    }

    #[test]
    fn wj_file_to_module_path_nested_and_mod_wj() {
        let base = std::path::Path::new("/proj/src_wj");
        let p = std::path::Path::new("/proj/src_wj/achievement/manager.wj");
        assert_eq!(
            wj_file_to_module_path(base, p),
            Some(vec![
                "achievement".to_string(),
                "manager".to_string()
            ])
        );
        let m = std::path::Path::new("/proj/src_wj/achievement/mod.wj");
        assert_eq!(
            wj_file_to_module_path(base, m),
            Some(vec!["achievement".to_string()])
        );
    }
}
