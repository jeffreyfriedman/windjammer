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
))]

//! HashMap::get binding into demoted `&str` formal must not emit `.clone()`.
//!
//! Ecosystem `wj-auth-api` config_from_toml:
//! ```
//! match map.get("jwt_ttl_secs") {
//!     Some(v) => parse_positive_int(v),  // text: string demoted to &str
//! }
//! ```
//! Tip emitted `parse_positive_int(v.clone())` → E0308 expected `&str`, found `String`.
//! Product interim: `digits_to_int("${v}")` forces a fresh owned string.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod cfg
pub mod use_cfg
"#;

const CFG: &str = r#"
use std::collections::HashMap
use std::strings

pub fn parse_digits(text: string) -> int {
    if strings.len(text) == 0 {
        return 0
    }
    let mut total = 0
    for ch in strings.chars(text) {
        if ch == '7' {
            total = total + 7
        }
        if ch == '2' {
            total = total + 2
        }
        if ch == '0' {
            total = total + 0
        }
    }
    total
}

pub fn ttl_from_map(map: HashMap<string, string>) -> int {
    match map.get("jwt_ttl_secs") {
        Some(v) => parse_digits(v),
        None => 3600,
    }
}
"#;

const USE_CFG: &str = r#"
use std::collections::HashMap
use crate::cfg::ttl_from_map

pub fn sample() -> int {
    let mut map = HashMap::new()
    map.insert("jwt_ttl_secs", "7200")
    ttl_from_map(map)
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("cfg.wj", CFG);
    test.add_file("use_cfg.wj", USE_CFG);
    test
}

#[test]
fn hashmap_get_binding_into_demoted_str_must_not_clone() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let cfg_rs = map.get("cfg.rs").expect("cfg.rs");
    eprintln!("cfg.rs:\n{cfg_rs}");

    let demoted = {
        let i = cfg_rs.find("fn parse_digits").unwrap_or(0);
        let sl = &cfg_rs[i..cfg_rs.len().min(i + 120)];
        sl.contains("text: &str") || sl.contains("text:&str")
    };

    if demoted {
        assert!(
            !cfg_rs.contains("parse_digits(v.clone())")
                && !cfg_rs.contains("parse_digits((v).clone())"),
            "demoted &str formal must not receive HashMap get binding .clone(). Got:\n{cfg_rs}"
        );
        assert!(
            cfg_rs.contains("parse_digits(v)")
                || cfg_rs.contains("parse_digits(&v)")
                || cfg_rs.contains("parse_digits(v.as_str())"),
            "expected borrow or move of get binding into demoted &str. Got:\n{cfg_rs}"
        );
    }
}
