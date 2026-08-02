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

//! Dogfood (types-crate): Pure demotes `name` to `&str`; Arrow keeps owned `String`
//! because `name` is stored. Match-arm call must `.to_string()`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn pure_str_arrow_owned_string_method_needs_to_string_at_call() {
    let source = r#"
pub struct PureBatch {
    pub n: i64,
}

pub struct ArrowBatch {
    pub last: string,
}

impl PureBatch {
    pub fn column_i64(self, name: string) -> i64 {
        if name == "customer_id" {
            return self.n
        }
        0
    }
}

impl ArrowBatch {
    pub fn column_i64(self, name: string) -> i64 {
        self.last = name
        0
    }
}

pub enum BatchData {
    Pure(PureBatch),
    Arrow(ArrowBatch),
}

pub fn batch_column_i64(data: BatchData, name: string) -> i64 {
    match data {
        BatchData::Pure(batch) => batch.column_i64(name),
        BatchData::Arrow(batch) => batch.column_i64(name),
    }
}

fn main() {
    let _ = batch_column_i64(BatchData::Pure(PureBatch { n: 1 }), "customer_id" + "")
}
"#;

    let rs = test_utils::compile_single(source);
    let outer_str = rs.contains("batch_column_i64") && rs.contains("name: &str");
    let arrow_owned = rs.contains("impl ArrowBatch")
        && (rs.contains("fn column_i64(&self, name: String)")
            || rs.contains("fn column_i64(&mut self, name: String)")
            || rs.contains("pub fn column_i64(&self, name: String)")
            || rs.contains("pub fn column_i64(&mut self, name: String)"));
    if outer_str && arrow_owned {
        assert!(
            rs.contains("column_i64(name.to_string())")
                || rs.contains("column_i64(name.clone())"),
            "outer &str → Arrow owned String must coerce. Got:\n{rs}"
        );
    }
    test_utils::verify_rust_compiles(&rs).expect("generated Rust must compile");
}
