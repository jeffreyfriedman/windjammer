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

//! REGRESSION (dogfood composition authenticate — E0382):
//!
//! `let x = match callee(Struct { field: local }) { ... }` parses as
//! `Let { value: Block { Match } }`. Match-only block codegen must use the
//! same auto_clone statement index as analysis (`enclosing_idx + 1`), or
//! the scrutinee move skips `.clone()` while a later use still clones (E0382).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn reused_string_in_match_scrutinee_struct_must_clone() {
    let source = r#"
pub struct AuthClaims {
    pub sub: string,
    pub tenant_slug: string,
    pub tenant_id: string,
    pub entity_id: string,
    pub email: string,
}

pub struct OperationContext {
    pub request_id: string,
    pub actor_sub: string,
    pub actor_email: string,
    pub surface: string,
}

fn unauthorized(msg: string) -> string { msg }
fn forbidden(msg: string) -> string { msg }
fn actor_email_from_claims(claims: AuthClaims) -> string { claims.email }
fn tenant_slug_from_claims(claims: AuthClaims) -> Result<string, string> {
    Ok(claims.tenant_slug)
}
fn default_rest_surface() -> string { "rest" }

struct Verifier {}
impl Verifier {
    fn verify(self, token: string) -> Result<AuthClaims, string> {
        Ok(AuthClaims {
            sub: "u1",
            tenant_slug: "demo",
            tenant_id: "t1",
            entity_id: "e1",
            email: "a@b.c",
        })
    }
}

struct AppDeps {
    token_verifier: Verifier,
}

pub fn operation_context_from_bearer(
    deps: AppDeps,
    bearer_token: string,
    request_id: string,
) -> Result<(string, OperationContext), string> {
    let claims = match deps.token_verifier.verify(bearer_token) {
        Ok(value) => value,
        Err(msg) => return Err(unauthorized(msg)),
    }

    let sub = claims.sub
    let tenant_slug_val = claims.tenant_slug
    let tenant_id = claims.tenant_id
    let entity_id = claims.entity_id
    let email = claims.email
    let claims_for_slug = AuthClaims {
        sub: sub,
        tenant_slug: tenant_slug_val,
        tenant_id: tenant_id,
        entity_id: entity_id,
        email: email,
    }
    let actor_email = actor_email_from_claims(claims_for_slug)
    let tenant_slug = match tenant_slug_from_claims(AuthClaims {
        sub: sub,
        tenant_slug: tenant_slug_val,
        tenant_id: tenant_id,
        entity_id: entity_id,
        email: email,
    }) {
        Ok(slug) => slug,
        Err(msg) => return Err(forbidden(msg)),
    }

    let ctx = OperationContext {
        request_id: request_id,
        actor_sub: sub,
        actor_email: actor_email,
        surface: default_rest_surface(),
    }

    Ok((tenant_slug, ctx))
}

fn main() {}
"#;

    let (generated, transpile_ok) = test_utils::compile_single_check(source);
    assert!(
        transpile_ok,
        "transpile must succeed. Got:\n{generated}"
    );

    // Middle AuthClaims (match scrutinee) must clone — bare `sub` then later
    // `sub.clone()` is E0382.
    let match_line = generated
        .lines()
        .find(|l| l.contains("tenant_slug_from_claims(AuthClaims"))
        .unwrap_or("");
    assert!(
        match_line.contains("sub.clone()") || match_line.contains("sub: sub.clone()"),
        "match-scrutinee AuthClaims must clone reused sub. Line:\n{match_line}\nFull:\n{generated}"
    );

    test_utils::verify_rust_compiles(&generated).unwrap_or_else(|err| {
        panic!("generated Rust must rustc (auto-clone). Err:\n{err}\nGenerated:\n{generated}");
    });
}
