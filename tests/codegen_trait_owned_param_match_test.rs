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
//! Trait methods declare owned `string` / custom structs, but impl methods are
//! sometimes codegen'd as `&String` / `&T` while the trait stays owned. Rustc:
//! expected signature `fn(..., String, ...)`
//! found signature `fn(..., &String, ...)`
//!
//! Seen on: `TokenIssuer::issue_scoped_token`, `ToolGatewayPort::log_invocation`,
//! `JournalEntryWriter::post`, `DomainEventWriter::append`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn trait_owned_string_params_must_match_impl_formals() {
    let source = r#"
pub struct AuthToken {
    pub access_token: string,
}

pub trait TokenIssuer {
    fn issue(self, user_id: string, email: string) -> Result<AuthToken, string>
}

pub struct JwtTokenIssuer {}

impl TokenIssuer for JwtTokenIssuer {
    fn issue(self, user_id: string, email: string) -> Result<AuthToken, string> {
        Ok(AuthToken {
            access_token: user_id + ":" + email,
        })
    }
}

fn main() {
    let issuer = JwtTokenIssuer {}
    let _ = issuer.issue("u1" + "", "a@b.c" + "")
}
"#;

    let generated = test_utils::compile_single(source);

    // Trait + impl formals must agree (both owned String, not impl-only &String).
    assert!(
        !generated.contains("fn issue(&self, user_id: &String")
            && !generated.contains("fn issue(self, user_id: &String")
            && !generated.contains("user_id: &String, email: &String"),
        "impl must not flip owned string formals to &String while trait stays owned. Got:\n{generated}"
    );
    assert!(
        generated.contains("user_id: String") && generated.contains("email: String"),
        "expected owned String formals on trait/impl. Got:\n{generated}"
    );
}

#[test]
fn trait_owned_struct_param_must_match_impl_formal() {
    let source = r#"
pub struct DomainEventDraft {
    pub name: string,
}

pub trait DomainEventWriter {
    fn append(self, event: DomainEventDraft) -> Result<(), string>
}

pub struct PostgresDomainEventWriter {}

impl DomainEventWriter for PostgresDomainEventWriter {
    fn append(self, event: DomainEventDraft) -> Result<(), string> {
        let _ = event.name + ""
        Ok(())
    }
}

fn main() {
    let w = PostgresDomainEventWriter {}
    let _ = w.append(DomainEventDraft { name: "posted" + "" })
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        !generated.contains("fn append(&self, event: &DomainEventDraft")
            && !generated.contains("fn append(self, event: &DomainEventDraft")
            && !generated.contains("event: &DomainEventDraft"),
        "impl must not flip owned struct formal to &T while trait stays owned. Got:\n{generated}"
    );
    assert!(
        generated.contains("event: DomainEventDraft"),
        "expected owned DomainEventDraft formal. Got:\n{generated}"
    );
}
