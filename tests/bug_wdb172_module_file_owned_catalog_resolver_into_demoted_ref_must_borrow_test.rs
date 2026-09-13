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
//! Prefer tip greens over dogfood/tip-cluster.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

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

fn wdb172_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("binder.wj", BINDER);
    test.add_file("exec.wj", EXEC);
    test
}

#[test]
fn wdb172_module_file_owned_resolver_into_demoted_ref_must_borrow() {
    let test = wdb172_fixture();
    let map = test
        .compile()
        .expect("WDB-172 multipass compile should succeed");
    let binder_rs = map.get("binder.rs").expect("binder.rs");
    let exec_rs = map.get("exec.rs").expect("exec.rs");

    eprintln!("WDB-172 binder.rs:\n{binder_rs}\nexec.rs:\n{exec_rs}");

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
            "WDB-172 RED: demoted &CatalogResolver received owned/clone. \
             Product: bind_ast(…, resolver) / resolve_select(resolver.clone(), …). Got:\n{exec_rs}\n{binder_rs}"
        );
    }

    if demoted_bind {
        assert!(
            !bad_bare || good,
            "WDB-172: demoted &CatalogResolver must borrow. Got:\n{exec_rs}"
        );
    }
}

fn wdb172_rel_gen_root() -> PathBuf {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    if tip.join("relational_sql_binder_port.rs").exists() {
        return tip;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen/relational")
}

fn wdb172_fn_formal_demoted_resolver(text: &str, fn_name: &str) -> bool {
    let needle = format!("fn {fn_name}");
    let Some(i) = text.find(&needle) else {
        return false;
    };
    let sl = &text[i..text.len().min(i + 220)];
    sl.contains("resolver: &RelationalCatalogResolver")
        || sl.contains("resolver:&RelationalCatalogResolver")
}

#[test]
fn wdb172_product_binder_must_borrow_resolver_into_demoted_formals() {
    let root = wdb172_rel_gen_root();
    let binder = root.join("relational_sql_binder_port.rs");
    let exec = root.join("relational_df_analytic_execute_port.rs");
    if !binder.exists() || !exec.exists() {
        eprintln!("WDB-172: skip product gate — binder/exec missing");
        return;
    }
    let binder_text = std::fs::read_to_string(&binder).expect("binder");
    let exec_text = std::fs::read_to_string(&exec).expect("exec");
    let bind_ast_demoted = wdb172_fn_formal_demoted_resolver(&binder_text, "relational_sql_bind_ast");
    let resolve_select_demoted = wdb172_fn_formal_demoted_resolver(&binder_text, "resolve_select");
    let resolve_update_demoted = wdb172_fn_formal_demoted_resolver(&binder_text, "resolve_update");
    let mut bad = Vec::new();
    if bind_ast_demoted
        && exec_text.contains("relational_sql_bind_ast(ast, resolver)")
        && !exec_text.contains("relational_sql_bind_ast(ast, &resolver)")
    {
        bad.push("exec: bind_ast(ast, resolver) without &".to_string());
    }
    for (i, line) in binder_text.lines().enumerate() {
        if resolve_select_demoted && line.contains("resolve_select(resolver.clone()") {
            bad.push(format!("{}: {}", i + 1, line.trim()));
        }
        if resolve_update_demoted && line.contains("resolve_update(resolver.clone()") {
            bad.push(format!("{}: {}", i + 1, line.trim()));
        }
    }
    eprintln!(
        "WDB-172 product bind_ast_demoted={} resolve_select_demoted={} bad={}",
        bind_ast_demoted,
        resolve_select_demoted,
        bad.len()
    );
    assert!(
        bad.is_empty(),
        "WDB-172 RED: product still passes owned/clone CatalogResolver into demoted & formal:\n{}",
        bad.join("\n")
    );
}
