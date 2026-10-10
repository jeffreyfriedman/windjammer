#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! WDB-172: owned CatalogResolver / `.clone()` into demoted `&CatalogResolver` must borrow.
//!
//! Product census (~6×):
//!   `relational_sql_bind_ast(ast, resolver)` while formal is `&RelationalCatalogResolver`
//!   `resolve_select(resolver.clone(), sel)` while formal is `&RelationalCatalogResolver`
//! → E0308 expected `&RelationalCatalogResolver`, found `RelationalCatalogResolver`.
//!
//! Distinct from WDB-167 (Provider / tuple field). Signature-driven — no hardcoded names.
//! Prefer tip greens over downstream/tip-cluster.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod binder
pub mod exec
"#;

const BINDER: &str = r#"
pub struct CatalogResolver {
    pub ok: bool,
    /// Non-Copy so tip demotes like product RelationalCatalogResolver (not Copy bool-only).
    pub label: string,
}

pub struct Ast {
    pub kind: int,
}

/// Read-only resolver consumer — tip demotes to `&CatalogResolver` (product bind_ast).
pub fn bind_ast(ast: Ast, resolver: CatalogResolver) -> bool {
    resolver.ok && resolver.label.len() > 0 && ast.kind >= 0
}

pub fn resolve_select(resolver: CatalogResolver, kind: int) -> bool {
    resolver.ok && resolver.label.len() > 0 && kind >= 0
}
"#;

const EXEC: &str = r#"
use crate::binder::Ast
use crate::binder::CatalogResolver
use crate::binder::bind_ast
use crate::binder::resolve_select

pub fn run(resolver: CatalogResolver) -> bool {
    let ast = Ast { kind: 1 }
    // Product: relational_sql_bind_ast(ast, resolver) into demoted &Resolver
    let a = bind_ast(ast, resolver.clone())
    // Product: resolve_select(resolver.clone(), …) into demoted &Resolver
    let b = resolve_select(resolver.clone(), 1)
    a && b
}

pub fn make_resolver() -> CatalogResolver {
    CatalogResolver { ok: true, label: "cap" }
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("binder.wj", BINDER);
    test.add_file("exec.wj", EXEC);
    test
}

#[test]
fn module_file_owned_resolver_into_demoted_ref_must_borrow() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let binder_rs = map.get("binder.rs").expect("binder.rs");
    let exec_rs = map.get("exec.rs").expect("exec.rs");

    eprintln!("binder.rs:\n{binder_rs}\nexec.rs:\n{exec_rs}");

    let demoted_bind = binder_rs.contains("resolver: &CatalogResolver")
        || binder_rs.contains("resolver:&CatalogResolver");
    let bad_bare = exec_rs.contains("bind_ast(ast, resolver)")
        && !exec_rs.contains("bind_ast(ast, &resolver)");
    let bad_clone = exec_rs.contains("resolve_select(resolver.clone()")
        && !exec_rs.contains("resolve_select(&resolver");
    let good = exec_rs.contains("bind_ast(ast, &resolver")
        || exec_rs.contains("resolve_select(&resolver");

    if demoted_bind && (bad_bare || bad_clone) && !good {
        panic!(
            "demoted &CatalogResolver received owned/clone. \
             Product: bind_ast(…, resolver) / resolve_select(resolver.clone(), …). Got:\n{exec_rs}\n{binder_rs}"
        );
    }

    if demoted_bind {
        assert!(
            !bad_bare || good,
            "demoted &CatalogResolver must borrow. Got:\n{exec_rs}"
        );
    }
}
