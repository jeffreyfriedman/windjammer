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

//! Gate (windjammer-ui StatusChip → Badge): exact dogfood pattern.
//!
//! Hand patch symptom: generated StatusChip did `Badge::new(&status)` (E0308 expected
//! String, found &String) or reused owned `status` after `variant_for(status)`.
//! Desired: pure `.wj` regenerate compiles without editing badge.rs / statuschip.rs.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn statuschip_badge_compose_should_compile_without_hand_patches() {
    let source = r#"
pub struct Badge {
    text: string,
}

impl Badge {
    pub fn new(text: string) -> Badge {
        Badge { text: text }
    }
    pub fn render(self) -> string {
        format!("<span>{}</span>", self.text)
    }
}

pub fn variant_for(status: string) -> int {
    if status.to_lowercase() == "paid" { 1 } else { 0 }
}

pub struct StatusChip {
    status: string,
}

impl StatusChip {
    pub fn new(status: string) -> StatusChip {
        StatusChip { status: status }
    }
    pub fn render(self) -> string {
        let status = self.status
        let _v = variant_for(status)
        Badge::new(status).render()
    }
}

fn main() {
    println!("{}", StatusChip::new("paid".to_string()).render())
}
"#;

    let result = test_utils::compile_single(source);
    let __result = test_utils::verify_rust_compiles(&result);
    assert!(
        __result.is_ok(),
        "StatusChip→Badge must compile from pure WJ (auto-clone or &str helpers). stderr={:?}\nGot:\n{}",
        __result.err(),
        result
    );
    assert!(
        !result.contains("Badge::new(&status)") && !result.contains("Badge::new(&"),
        "must not pass &String into Badge::new(String). Got:\n{}",
        result
    );
}
