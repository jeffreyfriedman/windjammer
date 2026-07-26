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

//! FAILING REPRO (product/visual): Alert must not emit emoji marks in HTML.
//!
//! LedgerKit / calm fintech UI rejects emoji in chrome. Codegen historically emitted
//! ❌⚠️ℹ️✅ in Alert::render. Desired: text marks ("Error", "Warning", …).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn alert_render_should_not_emit_emoji_marks() {
    let source = r#"
pub enum AlertVariant { Error, Warning, Info, Success }

pub struct Alert {
    message: string,
    variant: AlertVariant,
}

impl Alert {
    pub fn render(self) -> string {
        let mark = match self.variant {
            AlertVariant::Error => "Error",
            AlertVariant::Warning => "Warning",
            AlertVariant::Info => "Note",
            AlertVariant::Success => "Done",
        }
        format!("<div class='wj-alert'>{} {}</div>", mark, self.message)
    }
}

fn main() {
    let a = Alert { message: "x".to_string(), variant: AlertVariant::Error }
    let _ = a.render()
}
"#;

    let result = test_utils::compile_single(source);
    let has_emoji = result.contains("❌")
        || result.contains("⚠️")
        || result.contains("ℹ️")
        || result.contains("✅")
        || result.contains("\\u{");
    assert!(
        !has_emoji && result.contains("Error"),
        "Alert codegen should use text marks, not emoji. Got:\n{}",
        result
    );
}
