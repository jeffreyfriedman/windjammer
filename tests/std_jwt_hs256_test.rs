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

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn std_jwt_platform_shape_compiles() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "jwt_auth.wj",
        r#"
use std::jwt

pub fn verify_demo_token(token: string, secret: string) -> Result<string, string> {
    match jwt.verify_hs256(token, secret) {
        Ok(claims) => Ok(claims.tenant_slug),
        Err(msg) => Err(msg),
    }
}

pub fn mint_demo_token(secret: string) -> Result<string, string> {
    jwt.sign_hs256("dev", "demo", secret, 3600)
}
"#,
    );
    test.add_file("mod.wj", "pub mod jwt_auth");

    let map = test.compile().expect("compile");
    let rs = map.get("jwt_auth.rs").expect("rs");
    assert!(
        rs.contains("jwt::verify_hs256"),
        "verify must map to runtime jwt. Got:\n{rs}"
    );
    assert!(
        rs.contains("jwt::sign_hs256"),
        "sign must map to runtime jwt. Got:\n{rs}"
    );
    test.assert_compiles_without_error();
}
