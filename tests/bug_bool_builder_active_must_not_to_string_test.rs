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

//! P3.705 / LedgerKit `shell_chrome.wj`:
//! `ShellNavLink::active(is_active: bool)` must not emit
//! `.active(is_active.to_string())` (E0308). Same-crate builders are tip GREEN;
//! product fails when resolving against windjammer-ui (Tabs::active takes string).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const LINK: &str = r#"
pub struct ShellNavLink {
    pub active: bool,
    pub label: string,
}

impl ShellNavLink {
    pub fn new(label: string) -> ShellNavLink {
        ShellNavLink {
            active: false,
            label: label,
        }
    }

    pub fn active(self, active: bool) -> ShellNavLink {
        self.active = active
        self
    }
}

/// Ambiguous same-name method that takes string — must not steal bool builder.
pub struct Tabs {
    pub id: string,
}

impl Tabs {
    pub fn new() -> Tabs {
        Tabs { id: "" }
    }

    pub fn active(self, id: string) -> Tabs {
        self.id = id
        self
    }
}
"#;

const NAV: &str = r#"
use super::link::ShellNavLink

pub fn render_shell_nav(active_section: string) -> string {
    let is_active = active_section == "home"
    let link = ShellNavLink::new("Home").active(is_active)
    if link.active {
        "yes"
    } else {
        "no"
    }
}
"#;

const MOD: &str = r#"
mod link;
mod nav;
"#;

#[test]
fn bool_builder_active_must_not_to_string_with_string_overload() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("link.wj", LINK);
    test.add_file("nav.wj", NAV);
    let map = test.compile().expect("P3.705 active compile");
    let rs = map
        .get("nav.rs")
        .or_else(|| map.get("lib.rs"))
        .expect("nav.rs");
    eprintln!("P3.705 active nav.rs:\n{rs}");
    assert!(
        !rs.contains("is_active.to_string()")
            && !rs.contains(".active(is_active.to_string())"),
        "P3.705 RED: bool builder .active must not .to_string() the bool:\n{rs}"
    );
    assert!(
        rs.contains(".active(is_active)"),
        "P3.705 RED: expected bare bool into .active:\n{rs}"
    );
    test.cargo_check().expect("P3.705 active cargo-check");
}
