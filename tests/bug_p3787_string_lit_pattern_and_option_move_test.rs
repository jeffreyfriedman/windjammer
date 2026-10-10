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
    feature = "codegen_tests",
))]

//! P3.787: Oct 10 09:23 `wj` finance-screens regen (42 errors).
//!
//! `s.replace("&", "&amp;")` and `base.trim_end_matches("/")` emit
//! `String::from(...)`, which is not a `Pattern` and is not `&str`.
//! `match s.access_token { Some(t) => "Bearer " + t }` emits
//! `match &s.access_token { Some(mut t) => ... t ... }` and moves out of `&String`.
//! An owned `json` used by two `String` parsers emits `parse_buckets(json)` then
//! `parse_parties(json)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const REPLACE_SRC: &str = r#"
pub fn escape_html(s: string) -> string {
    s.replace("&", "&amp;").replace("<", "&lt;")
}

pub fn root(base: string) -> string {
    "${base.trim_end_matches("/")}"
}
"#;

const MATCH_SRC: &str = r#"
pub struct Session {
    pub access_token: Option<string>,
}

pub fn session_auth_header_value(s: Session) -> Option<string> {
    match s.access_token {
        Some(t) => {
            if t.len() > 0 {
                Some("Bearer " + t)
            } else {
                None
            }
        },
        None => None,
    }
}
"#;

const JSON_SRC: &str = r#"
pub fn parse_buckets(json: string) -> string {
    json
}

pub fn parse_parties(json: string) -> string {
    json
}

pub fn aging(kind: string, json: string) -> string {
    match kind {
        "move" => json,
        "aging" => {
            let fallback = "${json}"
            let buckets = parse_buckets("${json}")
            let parties = parse_parties(json)
            "${fallback}:${buckets}:${parties}"
        },
        _ => "other",
    }
}
"#;

#[test]
fn p3787_string_lit_replace_must_not_string_from() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", REPLACE_SRC);
    let map = test.compile().expect("P3.787 replace compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.787 replace lib.rs:\n{rs}");
    assert!(
        !rs.contains("String::from(\"&\")") && !rs.contains("String::from(\"/\")"),
        "P3.787 RED: string literal pattern wrapped in String::from:\n{rs}"
    );
}

#[test]
fn p3787_option_string_match_must_not_move_from_shared_ref() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", MATCH_SRC);
    let map = test.compile().expect("P3.787 match compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.787 match lib.rs:\n{rs}");
    assert!(
        rs.contains("fn session_auth_header_value(s: &Session)"),
        "P3.787: session receiver should demote to &Session:\n{rs}"
    );
    assert!(
        !rs.contains("Some(mut t)"),
        "P3.787 RED: Option<string> match moved a String out of a shared ref:\n{rs}"
    );
}

#[test]
fn p3787_owned_json_second_use_must_not_follow_move() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", JSON_SRC);
    let map = test.compile().expect("P3.787 json compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.787 json lib.rs:\n{rs}");
    assert!(
        rs.contains("fn parse_buckets(json: String)") && rs.contains("fn aging(kind: &str, json: String)"),
        "P3.787: parsers stay owned and aging keeps json: String:\n{rs}"
    );
    let buckets = rs.lines().find(|l| l.contains("parse_buckets(")).unwrap_or("");
    let parties = rs.lines().find(|l| l.contains("parse_parties(") && !l.contains("fn parse_parties")).unwrap_or("");
    let moved_then_used = buckets.contains("parse_buckets(json)") && parties.contains("parse_parties(json)");
    assert!(
        !moved_then_used,
        "P3.787 RED: json moved into parse_buckets then used again, buckets `{buckets}` parties `{parties}`:\n{rs}"
    );
}
