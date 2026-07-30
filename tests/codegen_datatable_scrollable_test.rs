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

//! FAILING REPRO (dogfood): DataTable.scrollable(true) must compile and
//! emit `lk-table-scroll` so sticky/zebra density CSS can wrap finance tables.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn datatable_scrollable_should_emit_lk_table_scroll() {
    let source = r#"
pub struct DataTable {
    scrollable: bool,
    body: string,
}

impl DataTable {
    pub fn new() -> DataTable {
        DataTable { scrollable: false, body: "<table></table>".to_string() }
    }
    pub fn scrollable(self, on: bool) -> DataTable {
        self.scrollable = on
        self
    }
    pub fn render(self) -> string {
        if self.scrollable {
            format!("<div class='lk-table-scroll'>{}</div>", self.body)
        } else {
            self.body
        }
    }
}

fn main() {
    let html = DataTable::new().scrollable(true).render()
    println!("{}", html)
}
"#;

    let result = test_utils::compile_single(source);
    let ok = result.contains("lk-table-scroll")
        && result.contains("scrollable")
        && !result.contains("error[E");
    assert!(
        ok,
        "DataTable.scrollable(true) should codegen a lk-table-scroll wrapper. Got:\n{}",
        result
    );
}
