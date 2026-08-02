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

//! Gate (dogfood): Table render must use themeable classes, not
//! hardcoded `padding: 12px` inline styles that fight density tokens.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn table_render_should_use_classes_not_fixed_padding() {
    let source = r#"
pub struct TableColumn { header: string }
pub struct TableRow { cells: Vec<string> }
pub struct Table {
    columns: Vec<TableColumn>,
    rows: Vec<TableRow>,
    striped: bool,
}

impl Table {
    pub fn render(self) -> string {
        let mut html = "<table class='wj-table'>".to_string()
        html.push_str("<thead><tr>")
        for col in &self.columns {
            html.push_str("<th>")
            html.push_str(&col.header)
            html.push_str("</th>")
        }
        html.push_str("</tr></thead><tbody>")
        for row in &self.rows {
            html.push_str("<tr>")
            for cell in &row.cells {
                html.push_str("<td>")
                html.push_str(cell)
                html.push_str("</td>")
            }
            html.push_str("</tr>")
        }
        html.push_str("</tbody></table>")
        html
    }
}

fn main() {
    let t = Table { columns: Vec::new(), rows: Vec::new(), striped: true }
    let _ = t.render()
}
"#;

    let result = test_utils::compile_single(source);
    let uses_class = result.contains("wj-table");
    let no_fixed_pad = !result.contains("padding: 12px") && !result.contains("padding:12px");
    assert!(
        uses_class && no_fixed_pad,
        "Table codegen should emit class-based cells, not padding: 12px. Got:\n{}",
        result
    );
}
