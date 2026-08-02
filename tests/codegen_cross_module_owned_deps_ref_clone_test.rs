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

//! Gate (dogfood):
//!
//! Cross-module: composition keeps `deps: AppDeps` owned, but routes multipass
//! emits `create_export_job(&deps.clone(), …)` → expected AppDeps, found &AppDeps.
//!
//! Simple single-file reuse often passes; platform needs multi-file + draft +
//! several string temps (matches analytics_use_cases / routes).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn cross_module_owned_deps_clone_must_not_add_ref() {
    let files = [
        (
            "composition.wj",
            r#"
pub struct AppDeps {
    pub tag: string,
}

pub struct ExportJobDraft {
    pub format: string,
}

pub fn create_export_job(
    deps: AppDeps,
    tenant_slug: string,
    draft: ExportJobDraft,
    request_id: string,
    actor_sub: string,
    actor_email: string,
) -> string {
    deps.tag + ":" + tenant_slug + ":" + draft.format + ":" + request_id + ":" + actor_sub + ":" + actor_email
}
"#,
        ),
        (
            "routes.wj",
            r#"
use composition::{AppDeps, ExportJobDraft, create_export_job}

pub struct Ctx {
    pub request_id: string,
    pub actor_sub: string,
    pub actor_email: string,
}

pub fn match_api_route(deps: AppDeps, tenant_slug: string, ctx: Ctx, draft: ExportJobDraft, which: int) -> string {
    if which == 1 {
        return create_export_job(deps, tenant_slug, draft, ctx.request_id, ctx.actor_sub, ctx.actor_email)
    }
    if which == 2 {
        return create_export_job(deps, tenant_slug, draft, ctx.request_id, ctx.actor_sub, ctx.actor_email)
    }
    create_export_job(deps, tenant_slug, draft, ctx.request_id, ctx.actor_sub, ctx.actor_email)
}
"#,
        ),
        (
            "main.wj",
            r#"
use composition::{AppDeps, ExportJobDraft}
use routes::{Ctx, match_api_route}

fn main() {
    let deps = AppDeps { tag: "x" + "" }
    let ctx = Ctx {
        request_id: "r" + "",
        actor_sub: "s" + "",
        actor_email: "e" + "",
    }
    let draft = ExportJobDraft { format: "json" + "" }
    let _ = match_api_route(deps, "demo" + "", ctx, draft, 1)
}
"#,
        ),
        ("mod.wj", "pub mod composition\npub mod routes\n"),
    ];

    let map = test_utils::compile_project(&files);
    let routes = map.get("routes.rs").cloned().unwrap_or_default();
    let composition = map.get("composition.rs").cloned().unwrap_or_default();

    assert!(
        composition.contains("fn create_export_job(deps: AppDeps")
            || composition.contains("fn create_export_job(mut deps: AppDeps"),
        "composition must keep owned AppDeps. Got:\n{composition}"
    );
    assert!(
        !routes.contains("create_export_job(&deps.clone()")
            && !routes.contains("create_export_job(&deps,"),
        "routes must pass owned deps.clone(), not &deps.clone() (dogfood). Got:\n{routes}"
    );
}
