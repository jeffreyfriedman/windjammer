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
//! Callee mutates owned `deps: AppDeps` fields (analyzer MutBorrowed) but emits
//! `mut deps: AppDeps` (owned). Multipass call sites that reuse `deps` must pass
//! `deps.clone()`, never `&deps.clone()` / `&mut deps`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn mutated_owned_deps_call_site_must_not_add_ref() {
    let source = r#"
pub struct ExportPort {}

impl ExportPort {
    fn create_job(self, draft: string) -> string {
        draft + ""
    }
}

pub struct AppDeps {
    pub analytics_export: ExportPort,
}

pub fn create_export_job(deps: AppDeps, tenant_slug: string, draft: string) -> string {
    let _ = tenant_slug
    deps.analytics_export.create_job(draft + "")
}

pub fn match_api_route(deps: AppDeps, which: int) -> string {
    if which == 1 {
        return create_export_job(deps, "one" + "", "d" + "")
    }
    if which == 2 {
        return create_export_job(deps, "two" + "", "d" + "")
    }
    create_export_job(deps, "fallback" + "", "d" + "")
}

fn main() {
    let deps = AppDeps { analytics_export: ExportPort {} }
    let _ = match_api_route(deps, 1)
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        generated.contains("fn create_export_job(deps: AppDeps")
            || generated.contains("fn create_export_job(mut deps: AppDeps"),
        "create_export_job must keep owned AppDeps formal. Got:\n{generated}"
    );
    assert!(
        !generated.contains("create_export_job(&deps")
            && !generated.contains("create_export_job(&mut deps"),
        "mutated owned AppDeps call sites must pass by value (deps.clone()), not &/&mut. Got:\n{generated}"
    );
}

#[test]
fn multipass_reused_owned_deps_must_not_mut_borrow() {
    use std::fs;
    use tempfile::TempDir;
    use windjammer::compiler::build_project;
    use windjammer::CompilationTarget;

    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("mod.wj"), "mod deps\nmod analytics\nmod routes\n").unwrap();
    fs::write(
        src.join("deps.wj"),
        r#"
pub struct ExportPort {}
impl ExportPort {
    fn create_job(self, draft: string) -> string { draft + "" }
}
pub struct AuditPort {}
impl AuditPort {
    fn append(self, tenant: string, event: string) -> Result<int, string> { Ok(1) }
}
pub struct AppDeps {
    pub analytics_export: ExportPort,
    pub audit_log_writer: AuditPort,
}
"#,
    )
    .unwrap();
    fs::write(
        src.join("analytics.wj"),
        r#"
use crate::deps::AppDeps
pub struct ExportJobDraft { pub format: string }
pub fn create_export_job(
    deps: AppDeps,
    tenant_slug: string,
    draft: ExportJobDraft,
    request_id: string,
    actor_sub: string,
    actor_email: string,
) -> Result<string, string> {
    let _ = tenant_slug
    let job = deps.analytics_export.create_job(draft.format + "")
    deps.audit_log_writer.append("demo" + "", request_id + actor_sub + actor_email + job)?
    Ok(job)
}
pub fn other_use(deps: AppDeps) -> int { 1 }
"#,
    )
    .unwrap();
    fs::write(
        src.join("routes.wj"),
        r#"
use crate::deps::{AppDeps, ExportPort, AuditPort}
use crate::analytics::{create_export_job, ExportJobDraft, other_use}
pub fn match_api_route(mut deps: AppDeps, which: int, body: string) -> string {
    let draft = ExportJobDraft { format: body + "" }
    let a = match create_export_job(deps, "t" + "", draft, "r" + "", "a" + "", "e" + "") {
        Ok(j) => j,
        Err(m) => m,
    }
    let b = match create_export_job(deps, "t2" + "", ExportJobDraft { format: "x" + "" }, "r2" + "", "a2" + "", "e2" + "") {
        Ok(j) => j,
        Err(m) => m,
    }
    let _ = other_use(deps)
    a + b
}
fn main() {
    let deps = AppDeps { analytics_export: ExportPort {}, audit_log_writer: AuditPort {} }
    let _ = match_api_route(deps, 1, "json" + "")
}
"#,
    )
    .unwrap();

    let out = tmp.path().join("build");
    build_project(&src.join("mod.wj"), &out, CompilationTarget::Rust, false).expect("compile");
    let routes = fs::read_to_string(out.join("routes.rs")).unwrap_or_default();
    let analytics = fs::read_to_string(out.join("analytics.rs")).unwrap_or_default();

    assert!(
        analytics.contains("fn create_export_job(mut deps: AppDeps")
            || analytics.contains("fn create_export_job(deps: AppDeps"),
        "callee must emit owned AppDeps. Got:\n{analytics}"
    );
    assert!(
        !routes.contains("create_export_job(&deps")
            && !routes.contains("create_export_job(&mut deps"),
        "multipass reused AppDeps must pass by value, not &/&mut. Got:\n{routes}"
    );
}
