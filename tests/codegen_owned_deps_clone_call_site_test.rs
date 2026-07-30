#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! FAILING REPRO (dogfood):
//!
//! Helpers take owned `deps: AppDeps`, but codegen demotes formals to `&AppDeps`
//! / `&mut AppDeps` and/or call sites pass `&deps` / `&deps.clone()` → E0308
//! when any layer still expects owned.
//!
//! Seen on: `create_export_job`, `post_journal_entry`, `reverse_journal_entry`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn owned_deps_formal_must_stay_owned_through_forward() {
    let source = r#"
pub struct AppDeps {
    pub tag: string,
}

pub fn create_export_job(deps: AppDeps, tenant_slug: string) -> string {
    deps.tag + ":" + tenant_slug
}

pub fn handle(deps: AppDeps, tenant_slug: string) -> string {
    create_export_job(deps, tenant_slug + "")
}

fn main() {
    let deps = AppDeps { tag: "x" + "" }
    let _ = handle(deps, "demo" + "")
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("fn create_export_job(deps: AppDeps")
            || generated.contains("fn create_export_job(mut deps: AppDeps"),
        "create_export_job must keep owned AppDeps formal. Got:\n{generated}"
    );
    assert!(
        !generated.contains("fn create_export_job(deps: &AppDeps")
            && !generated.contains("fn handle(deps: &AppDeps"),
        "owned AppDeps must not demote to &AppDeps. Got:\n{generated}"
    );
    assert!(
        !generated.contains("create_export_job(&deps")
            && !generated.contains("create_export_job(&deps.clone()"),
        "call site must pass owned AppDeps, not &deps. Got:\n{generated}"
    );
}

#[test]
fn owned_deps_chain_must_not_demote_to_ref() {
    let source = r#"
pub struct AppDeps {
    pub tag: string,
}

pub fn post_journal_entry(deps: AppDeps, tenant_slug: string) -> string {
    deps.tag + ":" + tenant_slug
}

pub fn reverse_journal_entry(deps: AppDeps, tenant_slug: string) -> string {
    let ctx_slug = tenant_slug + ""
    post_journal_entry(deps, ctx_slug + "")
}

fn main() {
    let deps = AppDeps { tag: "x" + "" }
    let _ = reverse_journal_entry(deps, "demo" + "")
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("fn reverse_journal_entry(deps: AppDeps")
            || generated.contains("fn reverse_journal_entry(mut deps: AppDeps"),
        "reverse must keep owned AppDeps formal. Got:\n{generated}"
    );
    assert!(
        generated.contains("fn post_journal_entry(deps: AppDeps")
            || generated.contains("fn post_journal_entry(mut deps: AppDeps"),
        "post must keep owned AppDeps formal. Got:\n{generated}"
    );
    assert!(
        !generated.contains("fn reverse_journal_entry(deps: &AppDeps")
            && !generated.contains("fn reverse_journal_entry(deps: &mut AppDeps")
            && !generated.contains("fn post_journal_entry(deps: &AppDeps")
            && !generated.contains("fn post_journal_entry(deps: &mut AppDeps"),
        "owned AppDeps chain must not demote to &/&mut. Got:\n{generated}"
    );
}
