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

//! FAILING REPRO (dogfood): compose KpiTile/KpiGrid in application Windjammer.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn home_should_compose_kpitile_grid() {
    let source = r##"
pub struct KpiTile { label: string, value_html: string }
pub struct KpiGrid { tiles: Vec<string> }

impl KpiTile {
    pub fn new(label: string) -> KpiTile {
        KpiTile { label: label, value_html: "".to_string() }
    }
    pub fn value_html(self, html: string) -> KpiTile {
        self.value_html = html
        self
    }
    pub fn render(self) -> string {
        format!("<div class='kpi'>{}{}</div>", self.label, self.value_html)
    }
}

impl KpiGrid {
    pub fn new() -> KpiGrid { KpiGrid { tiles: Vec::new() } }
    pub fn tile(self, html: string) -> KpiGrid {
        self.tiles.push(html)
        self
    }
    pub fn render(self) -> string {
        let mut body = "".to_string()
        for t in &self.tiles {
            body.push_str(t)
        }
        format!("<div class='kpi-grid'>{}</div>", body)
    }
}

fn home_kpis(cash: string, balanced: bool) -> string {
    let tb = if balanced {
        "Balanced".to_string()
    } else {
        "Out".to_string()
    }
    let a = KpiTile::new("Cash".to_string()).value_html(cash).render()
    let b = KpiTile::new("Trial".to_string()).value_html(tb).render()
    KpiGrid::new().tile(a).tile(b).render()
}

fn main() {
    println!("{}", home_kpis("$1".to_string(), true))
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("kpi-grid")
        && result.contains("KpiTile")
        && !result.contains("error[E");
    assert!(
        ok,
        "Home KPI composition (KpiTile/KpiGrid) should codegen. Got:\n{}",
        result
    );
}
