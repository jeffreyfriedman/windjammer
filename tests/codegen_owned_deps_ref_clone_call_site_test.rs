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

//! REGRESSION (dogfood):
//!
//! When owned `deps: AppDeps` is reused across branches/calls in one function,
//! codegen inserts `deps.clone()` but wraps it as `&deps.clone()` while the
//! callee still expects owned `AppDeps` → E0308.
//!
//! WJ source passes bare `deps`; generated Rust becomes `&deps.clone()`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn reused_owned_deps_clone_must_not_add_ref() {
    let source = r#"
pub struct AppDeps {
    pub tag: string,
}

pub fn create_export_job(deps: AppDeps, tenant_slug: string) -> string {
    deps.tag + ":" + tenant_slug
}

pub fn match_api_route(deps: AppDeps, which: int) -> string {
    if which == 1 {
        return create_export_job(deps, "one" + "")
    }
    if which == 2 {
        return create_export_job(deps, "two" + "")
    }
    create_export_job(deps, "fallback" + "")
}

fn main() {
    let deps = AppDeps { tag: "x" + "" }
    let _ = match_api_route(deps, 1)
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        !generated.contains("create_export_job(&deps.clone()")
            && !generated.contains("create_export_job(&deps,"),
        "reused owned deps must clone by value, not &deps.clone() (dogfood). Got:\n{generated}"
    );
}
