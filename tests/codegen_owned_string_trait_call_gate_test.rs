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

//! Gate — owned `string` trait call sites after ownership refresh.
//!
//! Trait methods take owned `string`. Call sites that pass:
//! - struct field access (`request.email` / `request.password`)
//! - format/concat temps (`tenant_slug + ""`)
//! must emit owned values that rustc accepts (clone when behind `&`, never bare `&field`).
//!
//! Known tip failure modes:
//! - rewrite `login(..., request: LoginRequest)` → `request: &LoginRequest`
//! - then emit `authenticate(request.email, request.password.clone())` → E0507 move of email
//! - historically also emitted `&request.password` / `close_period(&...)` → E0308

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn write_owned_string_trait_project(src: &std::path::Path) {
    fs::create_dir_all(src).unwrap();
    fs::write(
        src.join("ports.wj"),
        r#"
pub struct AuthenticatedUser {
    pub user_id: string,
}

pub trait CredentialAuthenticator {
    fn authenticate(self, email: string, password: string) -> Result<AuthenticatedUser, string>
}

pub trait PeriodLockPort {
    fn close_period(self, tenant_slug: string, period_id: string) -> Result<(), string>
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("adapters.wj"),
        r#"
use super::ports::{AuthenticatedUser, CredentialAuthenticator, PeriodLockPort}

pub struct SeedCredentialAuthenticator {}

impl CredentialAuthenticator for SeedCredentialAuthenticator {
    fn authenticate(self, email: string, password: string) -> Result<AuthenticatedUser, string> {
        if email.len() == 0 || password.len() == 0 {
            return Err("missing credentials")
        }
        Ok(AuthenticatedUser { user_id: email })
    }
}

pub struct SeedPeriodLock {}

impl PeriodLockPort for SeedPeriodLock {
    fn close_period(self, tenant_slug: string, period_id: string) -> Result<(), string> {
        if tenant_slug.len() == 0 || period_id.len() == 0 {
            return Err("missing period")
        }
        Ok(())
    }
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("composition.wj"),
        r#"
use super::ports::{CredentialAuthenticator, PeriodLockPort}
use super::adapters::{SeedCredentialAuthenticator, SeedPeriodLock}

pub struct LoginRequest {
    pub email: string,
    pub password: string,
}

pub struct AppDeps {
    pub credential_authenticator: SeedCredentialAuthenticator,
    pub period_lock: SeedPeriodLock,
}

pub fn login(deps: AppDeps, request: LoginRequest) -> Result<string, string> {
    let user = match deps.credential_authenticator.authenticate(request.email, request.password) {
        Ok(u) => u,
        Err(e) => return Err(e),
    }
    Ok(user.user_id)
}

pub fn close(deps: AppDeps, tenant_slug: string, period_id: string) -> Result<(), string> {
    // format/concat temps into owned string formals
    deps.period_lock.close_period(
        format!("{}{}", tenant_slug, ""),
        format!("{}{}", period_id, ""),
    )
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("mod.wj"),
        "pub mod ports\npub mod adapters\npub mod composition\n",
    )
    .unwrap();
}

#[test]
fn trait_owned_string_field_and_concat_must_cargo_check() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    write_owned_string_trait_project(&src);

    let wj = env!("CARGO_BIN_EXE_wj");
    let status = Command::new(wj)
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--module-file",
            "--library",
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .status()
        .expect("run wj");
    assert!(status.success(), "wj library build must succeed for repro sources");

    let login_rs = fs::read_to_string(out.join("composition.rs")).expect("composition.rs");
    assert!(
        !login_rs.contains("&request.password") && !login_rs.contains("&request.email"),
        "authenticate must not borrow request fields for owned string formals. Got:\n{login_rs}"
    );
    assert!(
        !login_rs.contains("close_period(&"),
        "close_period must not borrow owned format/concat temps. Got:\n{login_rs}"
    );

    let crate_dir = tmp.path().join("crate");
    fs::create_dir_all(crate_dir.join("src")).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        r#"[package]
name = "owned_string_trait_gate"
version = "0.0.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#,
    )
    .unwrap();

    let mut lib = String::from("#![allow(dead_code, unused_imports)]\n");
    for name in ["ports", "adapters", "composition"] {
        let body = fs::read_to_string(out.join(format!("{name}.rs"))).unwrap();
        lib.push_str(&format!("pub mod {name} {{\n{body}\n}}\n"));
    }
    fs::write(crate_dir.join("src/lib.rs"), lib).unwrap();

    let check = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&crate_dir)
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "owned string trait call sites must cargo check. stderr=\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
