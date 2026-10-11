//! Resolver scopes: imports, aliases, globs, re-exports, `crate::`/`super::`/`self::`, nesting.
#[path = "resolve_support.rs"]
mod support;
use support::*;
use windjammer::resolve::{DiagnosticKind, Res};

#[test]
fn explicit_import_and_alias() {
    let rp = resolve_files(&[
        ("", "mod util\nuse util::double\nuse util::triple as t3\nfn main() { let a = double(1); let b = t3(1) }"),
        ("util", "pub fn double(x: int) -> int { x }\npub fn triple(x: int) -> int { x }"),
    ]);
    assert_def(&rp, "main", "double", "util::double");
    assert_def(&rp, "main", "t3", "util::triple");
}

#[test]
fn braced_import_and_module_import() {
    let rp = resolve_files(&[
        ("", "mod util\nuse util::{double, triple}\nuse util\nfn main() { let a = double(1); let b = triple(1); let c = util::double(2) }"),
        ("util", "pub fn double(x: int) -> int { x }\npub fn triple(x: int) -> int { x }"),
    ]);
    assert_def(&rp, "main", "double", "util::double");
    assert_def(&rp, "main", "triple", "util::triple");
    assert_def(&rp, "main", "util::double", "util::double");
}

#[test]
fn glob_import_and_explicit_shadows_glob() {
    let rp = resolve_files(&[
        (
            "",
            "mod a\nmod b\nuse a::*\nuse b::pick\nfn main() { let x = only_a(); let y = pick() }",
        ),
        (
            "a",
            "pub fn only_a() -> int { 1 }\npub fn pick() -> int { 1 }",
        ),
        ("b", "pub fn pick() -> int { 2 }"),
    ]);
    assert_def(&rp, "main", "only_a", "a::only_a");
    assert_def(&rp, "main", "pick", "b::pick");
}

#[test]
fn two_globs_with_same_name_are_ambiguous() {
    let rp = resolve_files(&[
        (
            "",
            "mod a\nmod b\nuse a::*\nuse b::*\nfn main() { let y = pick() }",
        ),
        ("a", "pub fn pick() -> int { 1 }"),
        ("b", "pub fn pick() -> int { 2 }"),
    ]);
    assert!(matches!(res(&rp, "main", "pick"), Res::Ambiguous(v) if v.len() == 2));
    assert!(rp
        .report
        .diagnostics
        .iter()
        .any(|d| d.kind == DiagnosticKind::AmbiguousCall));
}

#[test]
fn local_item_conflicting_with_import_is_ambiguous() {
    let rp = resolve_files(&[
        ("", "mod a\nuse a::go\nfn go() {}\nfn main() { go() }"),
        ("a", "pub fn go() {}"),
    ]);
    assert!(matches!(res(&rp, "main", "go"), Res::Ambiguous(_)));
}

#[test]
fn pub_use_reexport_chain() {
    let rp = resolve_files(&[
        ("", "mod api\nuse api::helper\nfn main() { helper() }"),
        ("api", "mod inner\npub use inner::helper"),
        ("api::inner", "pub fn helper() {}"),
    ]);
    assert_def(&rp, "main", "helper", "api::inner::helper");
}

#[test]
fn reexport_through_glob_chain() {
    let rp = resolve_files(&[
        ("", "mod api\nuse api::*\nfn main() { helper() }"),
        ("api", "mod inner\npub use inner::*"),
        ("api::inner", "pub fn helper() {}"),
    ]);
    assert_def(&rp, "main", "helper", "api::inner::helper");
}

#[test]
fn crate_super_self_paths() {
    let rp = resolve_files(&[
        ("", "mod a\nfn top() {}\nfn main() {}"),
        (
            "a",
            "mod b\npub fn up() {}\nfn f() { crate::top(); b::deep(); }",
        ),
        (
            "a::b",
            "pub fn deep() { super::up(); crate::a::up(); }\nuse self::deep as d2\nfn g() { d2() }",
        ),
    ]);
    assert_def(&rp, "a::f", "crate::top", "top");
    assert_def(&rp, "a::f", "b::deep", "a::b::deep");
    assert_def(&rp, "a::b::deep", "super::up", "a::up");
    assert_def(&rp, "a::b::deep", "crate::a::up", "a::up");
    assert_def(&rp, "a::b::g", "d2", "a::b::deep");
}

#[test]
fn use_super_and_relative_imports() {
    let rp = resolve_files(&[
        ("", "mod a\nfn main() {}"),
        ("a", "mod b\npub fn up() {}"),
        (
            "a::b",
            "use super::up\nuse crate::a::up as up2\nfn f() { up(); up2() }",
        ),
    ]);
    assert_def(&rp, "a::b::f", "up", "a::up");
    assert_def(&rp, "a::b::f", "up2", "a::up");
}

#[test]
fn inline_nested_modules_with_same_names() {
    let rp = resolve_one(
        r#"
mod a { pub fn new() {} pub mod inner { pub fn new() {} pub fn go() { new() } } }
mod b { pub fn new() {} }
fn main() { a::new(); a::inner::new(); b::new() }
"#,
    );
    assert_def(&rp, "main", "a::new", "a::new");
    assert_def(&rp, "main", "a::inner::new", "a::inner::new");
    assert_def(&rp, "main", "b::new", "b::new");
    assert_def(&rp, "a::inner::go", "new", "a::inner::new");
}

#[test]
fn imported_type_assoc_and_enum_variants() {
    let rp = resolve_files(&[
        ("", "mod m\nuse m::Shape\nuse m::Color::*\nfn main() { let a = Shape::make(); let b = Red(1); let c = m::Color::Red(2) }"),
        ("m", "pub struct Shape {}\nimpl Shape { pub fn make() -> Shape { Shape {} } }\npub enum Color { Red(int), Blue }"),
    ]);
    assert_def(&rp, "main", "Shape::make", "m::Shape::make");
    assert_def(&rp, "main", "Red", "m::Color::Red");
    assert_def(&rp, "main", "m::Color::Red", "m::Color::Red");
}

#[test]
fn type_alias_resolves_assoc_fns() {
    let rp = resolve_one(
        "struct S {}\nimpl S { fn mk() -> S { S {} } }\ntype Alias = S\nfn main() { Alias::mk() }",
    );
    assert_def(&rp, "main", "Alias::mk", "S::mk");
}

#[test]
fn unresolvable_internal_import_is_diagnosed_external_is_not() {
    let rp = resolve_files(&[
        (
            "",
            "mod a\nuse a::missing\nuse std::fs\nuse serde::Json\nfn main() {}",
        ),
        ("a", "pub fn present() {}"),
    ]);
    let bad: Vec<_> = rp
        .report
        .diagnostics
        .iter()
        .filter(|d| d.kind == DiagnosticKind::BadImport)
        .collect();
    assert_eq!(bad.len(), 1, "{bad:?}");
}

#[test]
fn import_cycles_terminate() {
    let rp = resolve_files(&[
        ("", "mod a\nmod b\nfn main() { x() }\nuse a::*"),
        ("a", "use b::*"),
        ("b", "use a::*"),
    ]);
    assert!(rp.report.unresolved >= 1);
}

#[test]
fn module_path_from_file_layout() {
    use std::path::Path;
    use windjammer::resolve::module_path_from_file as m;
    let root = Path::new("/p/src");
    assert_eq!(m(root, Path::new("/p/src/main.wj")), Vec::<String>::new());
    assert_eq!(m(root, Path::new("/p/src/a/b.wj")), vec!["a", "b"]);
    assert_eq!(m(root, Path::new("/p/src/a/mod.wj")), vec!["a"]);
}
