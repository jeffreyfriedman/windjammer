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

//! P3.791: a cross-module `"${json}"` passed into an owned `String` parser must
//! clone when a later call still uses `json`.
//!
//! Oct 10 10:16 `wj` finance-screens regen is one E0382:
//! `parse_aging_bucket_fields(json)` then `parse_aging_party_line_fields(json.clone())`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const JSON: &str = r#"
pub struct Bucket {
    pub label: string,
}

pub struct Party {
    pub name: string,
}

pub fn parse_aging_bucket_fields(json: string) -> Vec<Bucket> {
    if json.len() == 0 {
        vec![]
    } else {
        vec![Bucket { label: json }]
    }
}

pub fn parse_aging_party_line_fields(json: string) -> Vec<Party> {
    if json.len() == 0 {
        vec![]
    } else {
        vec![Party { name: json }]
    }
}

pub fn aging_chart_title_from_json(json: string) -> string {
    if json.len() == 0 { "t" } else { "AR" }
}

pub fn aging_kind_wire_from_json(json: string) -> string {
    if json.len() == 0 { "ar" } else { "ar" }
}

pub fn write_check_tags_from_json(json: string) -> string {
    json
}

pub fn parse_migration_job_fields(json: string) -> string {
    json
}
"#;

const READ: &str = r#"
use super::json::{
    parse_aging_bucket_fields,
    parse_aging_party_line_fields,
    aging_chart_title_from_json,
    aging_kind_wire_from_json,
    write_check_tags_from_json,
    parse_migration_job_fields,
}

pub fn render_read_model(kind: string, json: string) -> string {
    match kind {
        "write-check-tags" => write_check_tags_from_json(json),
        "aging" => {
            let fallback = "${json}"
            let title = aging_chart_title_from_json("${json}")
            let wire = aging_kind_wire_from_json("${json}")
            let buckets = parse_aging_bucket_fields("${json}")
            let parties = parse_aging_party_line_fields(json)
            "${fallback}:${title}:${wire}:${buckets.len()}:${parties.len()}"
        },
        "migrations" => parse_migration_job_fields(json),
        _ => "other",
    }
}
"#;

#[test]
fn p3791_cross_module_json_interp_must_clone_before_later_use() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", "mod json;\nmod read_models;\n");
    test.add_file("json.wj", JSON);
    test.add_file("read_models.wj", READ);
    let map = test.compile().expect("P3.791 compile");
    let read = map.get("read_models.rs").expect("read_models.rs");
    let json = map.get("json.rs").expect("json.rs");
    eprintln!("P3.791 json.rs:\n{json}\nP3.791 read_models.rs:\n{read}");
    assert!(
        json.contains("fn parse_aging_bucket_fields(json: String)")
            && read.contains("fn render_read_model(kind: &str, json: String)"),
        "P3.791: bucket parser stays owned and render keeps json: String:\n{json}\n{read}"
    );
    let buckets = read
        .lines()
        .find(|l| l.contains("parse_aging_bucket_fields("))
        .unwrap_or("");
    let parties = read
        .lines()
        .find(|l| l.contains("parse_aging_party_line_fields("))
        .unwrap_or("");
    assert!(
        buckets.contains("json.clone()") || buckets.contains("&json"),
        "P3.791 RED: moved json into parse_aging_bucket_fields before a later use, buckets `{buckets}` parties `{parties}`:\n{read}"
    );
}
