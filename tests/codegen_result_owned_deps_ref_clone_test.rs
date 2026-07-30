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

//! FAILING REPRO (dogfood)` — E0308):
//!
//! Platform emits:
//! `match { let _temp0 = format!(…); create_export_job(&deps.clone(), &_temp0, …) }`
//! while composition keeps `deps: AppDeps` owned.
//!
//! The `?` / Result path wraps the call in `match { … }` on routes (~8 of ~17
//! remaining platform errors on WJ tip `5ade3f99`).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn result_question_mark_owned_deps_must_not_ref_clone() {
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
) -> Result<string, string> {
    Ok(deps.tag + ":" + tenant_slug + ":" + draft.format + ":" + request_id + ":" + actor_sub + ":" + actor_email)
}

pub fn post_journal_entry(deps: AppDeps, tenant_slug: string) -> Result<string, string> {
    Ok(deps.tag + ":" + tenant_slug)
}
"#,
        ),
        (
            "routes.wj",
            r#"
use composition::{AppDeps, ExportJobDraft, create_export_job, post_journal_entry}

pub struct Ctx {
    pub request_id: string,
    pub actor_sub: string,
    pub actor_email: string,
}

pub fn handle_export(deps: AppDeps, tenant_slug: string, ctx: Ctx, draft: ExportJobDraft) -> Result<string, string> {
    let out = create_export_job(deps, tenant_slug, draft, ctx.request_id, ctx.actor_sub, ctx.actor_email)?
    Ok(out)
}

pub fn handle_post(deps: AppDeps, tenant_slug: string) -> Result<string, string> {
    let out = post_journal_entry(deps, tenant_slug)?
    Ok(out)
}
"#,
        ),
        (
            "main.wj",
            r#"
use composition::{AppDeps, ExportJobDraft}
use routes::{Ctx, handle_export, handle_post}

fn main() {
    let deps = AppDeps { tag: "x" + "" }
    let ctx = Ctx {
        request_id: "r" + "",
        actor_sub: "s" + "",
        actor_email: "e" + "",
    }
    let draft = ExportJobDraft { format: "json" + "" }
    let _ = handle_export(deps, "demo" + "", ctx, draft)
    let deps2 = AppDeps { tag: "y" + "" }
    let _ = handle_post(deps2, "demo" + "")
}
"#,
        ),
        ("mod.wj", "pub mod composition\npub mod routes\n"),
    ];

    let map = test_utils::compile_project(&files);
    let routes = map.get("routes.rs").cloned().unwrap_or_default();
    let composition = map.get("composition.rs").cloned().unwrap_or_default();

    assert!(
        composition.contains("deps: AppDeps")
            && !composition.contains("create_export_job(deps: &AppDeps"),
        "composition must keep owned AppDeps. Got:\n{composition}"
    );
    assert!(
        !routes.contains("&deps.clone()")
            && !routes.contains("create_export_job(&deps")
            && !routes.contains("post_journal_entry(&deps"),
        "Result/? call sites must not emit &deps.clone() (dogfood). Got:\n{routes}"
    );
}
