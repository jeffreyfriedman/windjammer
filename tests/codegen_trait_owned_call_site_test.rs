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
//! Trait methods declare owned `string` / custom structs. Call sites must pass
//! owned values (or `.clone()`), not `&arg` / `&arg.to_string()` mismatches.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn trait_owned_string_args_must_not_borrow_at_call_site() {
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
    let uid = "u1" + ""
    let mail = "a@b.c" + ""
    let _ = issuer.issue(uid, mail)
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        !generated.contains("issue(&uid")
            && !generated.contains("issue(&mail")
            && !generated.contains(".issue(&"),
        "owned trait string args must not be borrowed at call site. Got:\n{generated}"
    );
    assert!(
        generated.contains("user_id: String") && generated.contains("email: String"),
        "expected owned String formals. Got:\n{generated}"
    );
}

#[test]
fn trait_owned_struct_arg_must_not_borrow_at_call_site() {
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
    let draft = DomainEventDraft { name: "posted" + "" }
    let _ = w.append(draft)
}
"#;

    let generated = test_utils::compile_single(source);

    assert!(
        !generated.contains("append(&draft") && !generated.contains(".append(&"),
        "owned trait struct arg must not be borrowed at call site. Got:\n{generated}"
    );
    assert!(
        generated.contains("event: DomainEventDraft"),
        "expected owned DomainEventDraft formal. Got:\n{generated}"
    );
}
