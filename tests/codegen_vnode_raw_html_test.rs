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

//! FAILING REPRO (dogfood): VNode.raw_html trusted fragments for #main paint.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn vnode_raw_html_should_codegen() {
    let source = r##"
pub enum VNode {
    Text(string),
    RawHtml(string),
}

impl VNode {
    pub fn raw_html(html: string) -> VNode {
        VNode::RawHtml(html)
    }
}

fn main() {
    let node = VNode::raw_html("<div class=\"home-hero\">ok</div>".to_string())
    match node {
        VNode::RawHtml(html) => println!("{}", html),
        VNode::Text(_) => println!("text"),
    }
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("raw_html")
        && result.contains("RawHtml")
        && !result.contains("error[E");
    assert!(
        ok,
        "VNode::raw_html should codegen cleanly. Got:\n{}",
        result
    );
}
