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
    feature = "integration_tests",
))]

//! P3.532 leftover: method keeps owned `authorization: String` (caller chain) while
//! free `resolve_token` demotes to `&str`. Call site must emit `&authorization`.
//!
//! AST stub `string` must not beat preregistered demoted formals (E0308).

use std::process::Command;
use tempfile::TempDir;

#[test]
fn owned_method_param_into_demoted_resolve_token_must_borrow() {
    let dir = TempDir::new().expect("temp");
    let src = dir.path().join("src");
    std::fs::create_dir_all(src.join("domain")).unwrap();
    std::fs::write(src.join("mod.wj"), "pub mod domain\n").unwrap();
    std::fs::write(src.join("domain").join("mod.wj"), "pub mod auth\n").unwrap();
    std::fs::write(
        src.join("domain").join("auth.wj"),
        r#"use std::strings

pub struct AuthApp {
    pub n: int,
}

pub struct HttpReply {
    pub body: string,
}

impl AuthApp {
    pub fn new() -> AuthApp {
        AuthApp { n: 0 }
    }

    fn dispatch(self, authorization: string, cookie: string) -> HttpReply {
        self.profile(authorization, cookie)
    }

    fn profile(self, authorization: string, cookie: string) -> HttpReply {
        let token = resolve_token(authorization, cookie)
        if strings.len(token) == 0 {
            return HttpReply { body: "missing" }
        }
        HttpReply { body: token }
    }
}

fn resolve_token(authorization: string, cookie: string) -> string {
    let bearer = bearer_token(authorization)
    if strings.len(bearer) > 0 {
        return bearer
    }
    cookie
}

fn bearer_token(authorization: string) -> string {
    strings.trim(authorization)
}
"#,
    )
    .unwrap();

    let out = dir.path().join("out");
    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--module-file",
            "--no-cargo",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let auth = std::fs::read_to_string(out.join("domain").join("auth.rs")).expect("auth.rs");
    let resolve_sig = auth
        .lines()
        .find(|l| l.contains("fn resolve_token("))
        .unwrap_or("")
        .to_string();
    let resolve_call = auth
        .lines()
        .find(|l| l.contains("resolve_token(") && !l.contains("fn resolve_token"))
        .unwrap_or("")
        .to_string();
    let profile_sig = auth
        .lines()
        .find(|l| l.contains("fn profile("))
        .unwrap_or("")
        .to_string();

    // Emit-truth: either both sides demote to `&str` (bare pass OK), or the
    // method keeps owned `String` and must borrow into demoted resolve_token.
    if resolve_sig.contains("authorization: &str")
        && profile_sig.contains("authorization: String")
    {
        assert!(
            resolve_call.contains("resolve_token(&authorization"),
            "P3.532 RED: owned method param into demoted &str must borrow:\n{profile_sig}\n{resolve_sig}\n{resolve_call}\n{auth}"
        );
    }
    assert!(
        !resolve_sig.contains("authorization: &str")
            || !profile_sig.contains("authorization: String")
            || resolve_call.contains("resolve_token(&authorization"),
        "P3.532 RED: String formal into &str resolve_token without borrow:\n{profile_sig}\n{resolve_sig}\n{resolve_call}"
    );
}
