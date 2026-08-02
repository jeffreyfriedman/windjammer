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

//! Gate (dogfood): BusinessWorkspace bank slot must codegen.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn business_workspace_pane_should_codegen() {
    let source = r##"
pub struct BusinessWorkspace {
    layout: string,
    rail_html: string,
    register_html: string,
    write_check_html: string,
    memorized_html: string,
    bank_html: string,
}

impl BusinessWorkspace {
    pub fn new() -> BusinessWorkspace {
        BusinessWorkspace {
            layout: "pane".to_string(),
            rail_html: "".to_string(),
            register_html: "".to_string(),
            write_check_html: "".to_string(),
            memorized_html: "".to_string(),
            bank_html: "".to_string(),
        }
    }
    pub fn layout(self, layout: string) -> BusinessWorkspace {
        self.layout = layout
        self
    }
    pub fn rail_html(self, html: string) -> BusinessWorkspace {
        self.rail_html = html
        self
    }
    pub fn register_html(self, html: string) -> BusinessWorkspace {
        self.register_html = html
        self
    }
    pub fn write_check_html(self, html: string) -> BusinessWorkspace {
        self.write_check_html = html
        self
    }
    pub fn memorized_html(self, html: string) -> BusinessWorkspace {
        self.memorized_html = html
        self
    }
    pub fn bank_html(self, html: string) -> BusinessWorkspace {
        self.bank_html = html
        self
    }
    pub fn render(self) -> string {
        "<div class=\"wj-business-workspace\" data-layout=\"pane\" data-wj-business-workspace>".to_string()
            + "<aside class=\"wj-bw-rail\">"
            + self.rail_html
            + "</aside><section class=\"wj-bw-register\">"
            + self.register_html
            + "</section><section class=\"wj-bw-write\">"
            + self.write_check_html
            + "</section><section class=\"wj-bw-memorized\">"
            + self.memorized_html
            + "</section><section class=\"wj-bw-bank\">"
            + self.bank_html
            + "</section></div>"
    }
}

fn main() {
    println!("{}", BusinessWorkspace::new()
        .layout("pane".to_string())
        .rail_html("RAIL".to_string())
        .register_html("REG".to_string())
        .write_check_html("WC".to_string())
        .memorized_html("MEM".to_string())
        .bank_html("BANK".to_string())
        .render())
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("wj-business-workspace")
        && result.contains("data-layout")
        && result.contains("wj-bw-rail")
        && result.contains("wj-bw-register")
        && result.contains("wj-bw-write")
        && result.contains("wj-bw-memorized")
        && result.contains("wj-bw-bank")
        && !result.contains("error[E");
    assert!(
        ok,
        "BusinessWorkspace pane+bank compose should codegen. Got:\n{}",
        result
    );
}
