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

//! Gate (dogfood): KpiTile.value_html + KpiGrid.tile
//! chaining must codegen without &String / ownership errors and emit wj-kpi-* classes.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn kpitile_grid_chain_should_emit_wj_kpi_classes() {
    let source = r#"
pub struct KpiTile {
    label: string,
    value_html: string,
}

impl KpiTile {
    pub fn new(label: string) -> KpiTile {
        KpiTile { label: label, value_html: "".to_string() }
    }
    pub fn value_html(self, html: string) -> KpiTile {
        self.value_html = html
        self
    }
    pub fn render(self) -> string {
        format!("<div class='wj-kpi-tile'>{}{}</div>", self.label, self.value_html)
    }
}

pub struct KpiGrid {
    tiles: Vec<string>,
}

impl KpiGrid {
    pub fn new() -> KpiGrid {
        KpiGrid { tiles: Vec::new() }
    }
    pub fn tile(self, html: string) -> KpiGrid {
        self.tiles.push(html)
        self
    }
    pub fn render(self) -> string {
        let mut body = "".to_string()
        for t in &self.tiles {
            body.push_str(t)
        }
        format!("<div class='wj-kpi-grid'>{}</div>", body)
    }
}

fn main() {
    let tile = KpiTile::new("Cash".to_string()).value_html("$1.00".to_string()).render()
    let html = KpiGrid::new().tile(tile).render()
    println!("{}", html)
}
"#;

    let result = test_utils::compile_single(source);
    let ok = result.contains("wj-kpi-tile")
        && result.contains("wj-kpi-grid")
        && result.contains("fn value_html")
        && !result.contains("error[E");
    assert!(
        ok,
        "KpiTile/KpiGrid chain should codegen wj-kpi classes without errors. Got:\n{}",
        result
    );
}
