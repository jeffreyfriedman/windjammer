#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! TDD: Cross-module struct ownership inference for app-style architectures.
//!
//! Pattern: A composition root struct (Config) is passed to handler functions
//! that only read from it via field access. The analyzer should infer Borrowed
//! (&Config), not Owned (Config), so call sites that hold &mut refs can pass
//! them without type mismatch.

#[test]
fn cross_module_trait_field_access_inferred_borrowed() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    let ports = src.join("ports");
    let adapters = src.join("adapters");
    let composition = src.join("composition");
    std::fs::create_dir_all(&ports).unwrap();
    std::fs::create_dir_all(&adapters).unwrap();
    std::fs::create_dir_all(&composition).unwrap();

    // Port trait
    std::fs::write(
        ports.join("auth.wj"),
        r#"
pub struct AuthClaims {
    pub sub: string,
    pub email: string,
}

pub trait TokenVerifier {
    fn verify(self, bearer_token: string) -> Result<AuthClaims, string>
}

pub trait TokenIssuer {
    fn issue(self, sub: string, email: string) -> Result<string, string>
}
"#,
    )
    .unwrap();

    std::fs::write(ports.join("mod.wj"), "pub mod auth\n").unwrap();

    // Adapter impls (with non-Copy string fields so auto-Copy doesn't trigger)
    std::fs::write(
        adapters.join("jwt_verifier.wj"),
        r#"
use crate::ports::auth::{AuthClaims, TokenVerifier}

pub struct JwtVerifier {
    pub secret: string,
}

impl TokenVerifier for JwtVerifier {
    fn verify(self, bearer_token: string) -> Result<AuthClaims, string> {
        if bearer_token.is_empty() {
            return Err("empty token")
        }
        Ok(AuthClaims { sub: "u1".to_string(), email: "t@t.com".to_string() })
    }
}
"#,
    )
    .unwrap();

    std::fs::write(
        adapters.join("jwt_issuer.wj"),
        r#"
use crate::ports::auth::TokenIssuer

pub struct JwtIssuer {
    pub secret: string,
}

impl TokenIssuer for JwtIssuer {
    fn issue(self, sub: string, email: string) -> Result<string, string> {
        Ok("token-" + sub + "-" + email)
    }
}
"#,
    )
    .unwrap();

    std::fs::write(
        adapters.join("mod.wj"),
        "pub mod jwt_verifier\npub mod jwt_issuer\n",
    )
    .unwrap();

    // Composition root
    std::fs::write(
        composition.join("deps.wj"),
        r#"
use crate::adapters::jwt_verifier::JwtVerifier
use crate::adapters::jwt_issuer::JwtIssuer

pub struct AppDeps {
    pub verifier: JwtVerifier,
    pub issuer: JwtIssuer,
}
"#,
    )
    .unwrap();

    std::fs::write(
        composition.join("login.wj"),
        r#"
use crate::composition::deps::AppDeps
use crate::ports::auth::TokenVerifier
use crate::ports::auth::TokenIssuer

pub fn login(deps: AppDeps, token: string) -> Result<string, string> {
    let claims = match deps.verifier.verify(token) {
        Ok(value) => value,
        Err(msg) => return Err(msg),
    }
    deps.issuer.issue(claims.sub, claims.email)
}
"#,
    )
    .unwrap();

    std::fs::write(
        composition.join("mod.wj"),
        "pub mod deps\npub mod login\n",
    )
    .unwrap();

    std::fs::write(
        src.join("mod.wj"),
        "pub mod ports\npub mod adapters\npub mod composition\n",
    )
    .unwrap();

    std::fs::write(
        dir.path().join("wj.toml"),
        "[package]\nname = \"test_app_own\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    let wj = std::path::PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj)
        .arg("build")
        .arg("--no-cargo")
        .arg(&src)
        .current_dir(dir.path())
        .output()
        .expect("wj build failed");

    assert!(
        output.status.success(),
        "Build failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let build_dir = dir.path().join("build");
    let login_rs = std::fs::read_to_string(
        build_dir.join("composition").join("login.rs"),
    )
    .expect("login.rs not found");
    println!("login.rs:\n{}", login_rs);

    // deps is only read (field access + trait method calls that take &self),
    // so it should be borrowed.
    assert!(
        login_rs.contains("deps: &AppDeps"),
        "Expected deps: &AppDeps (borrowed) for read-only field access.\nGenerated:\n{}",
        login_rs,
    );
}

#[test]
fn cross_module_string_param_str_ref_optimization() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();

    std::fs::write(
        src.join("utils.wj"),
        r#"
pub fn greet(name: string) -> string {
    format!("Hello, {}!", name)
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("caller.wj"),
        r#"
use crate::utils::greet

pub fn welcome(user: string) -> string {
    greet(user)
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("mod.wj"),
        "pub mod utils\npub mod caller\n",
    )
    .unwrap();

    std::fs::write(
        dir.path().join("wj.toml"),
        "[package]\nname = \"test_str_ref\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    let wj = std::path::PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj)
        .arg("build")
        .arg("--no-cargo")
        .arg(&src)
        .current_dir(dir.path())
        .output()
        .expect("wj build failed");

    assert!(
        output.status.success(),
        "Build failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let build_dir = dir.path().join("build");

    let utils_rs = std::fs::read_to_string(build_dir.join("utils.rs"))
        .expect("utils.rs not found");
    println!("utils.rs:\n{}", utils_rs);

    let caller_rs = std::fs::read_to_string(build_dir.join("caller.rs"))
        .expect("caller.rs not found");
    println!("caller.rs:\n{}", caller_rs);

    // greet only reads name (format!), so &str optimization should apply.
    // The formal and call site must agree.
    let formal_is_str_ref = utils_rs.contains("fn greet(name: &str)");
    let formal_is_string = utils_rs.contains("fn greet(name: String)")
        || utils_rs.contains("fn greet(mut name: String)");

    // Either &str or String is fine, but the call site must match.
    if formal_is_str_ref {
        // Call site should NOT add .to_string() for &str param
        assert!(
            !caller_rs.contains(".to_string()"),
            "If formal is &str, call site should not add .to_string().\ncaller.rs:\n{}",
            caller_rs,
        );
    } else if formal_is_string {
        // Call site should NOT add & for String param
        assert!(
            !caller_rs.contains("greet(&"),
            "If formal is String, call site should not add &.\ncaller.rs:\n{}",
            caller_rs,
        );
    } else {
        panic!(
            "greet formal must be either &str or String.\nutils.rs:\n{}",
            utils_rs,
        );
    }
}
