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

//! P3.522: a caller that reads `self.config` then calls `&mut self` `check_rate`
//! must itself be `&mut self` (and must not clone `self` to call it).
//!
//! Product `wj-notes-api` defines `handle_http` / `handle_method` *above*
//! `check_rate`. Adapter `Ok(mut app) => app.handle_http(...)` cannot move
//! `NotesApp` out of `MutexGuard` (E0507). Isolates that define `check_rate`
//! first emit `handle_method(&mut self)` (false-GREEN). Do not reshape the app.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
use std::collections::HashMap
use std::sync::{Arc, Mutex}

pub struct Config {
    pub log_level: string,
}

pub struct App {
    pub buckets: HashMap<string, i32>,
    pub config: Config,
}

fn take_map(buckets: HashMap<string, i32>) -> HashMap<string, i32> {
    buckets
}

fn put_map(buckets: HashMap<string, i32>) -> HashMap<string, i32> {
    buckets
}

impl App {
    pub fn handle_http(self, key: string) -> i32 {
        self.handle_method(key)
    }

    pub fn handle_method(self, key: string) -> i32 {
        let level = self.config.log_level
        if self.check_rate(key) {
            return 429
        }
        let n = self.dispatch()
        let _ = level
        n
    }

    fn check_rate(self, key: string) -> bool {
        let next = put_map(take_map(self.buckets))
        self.buckets = next
        key.len() == 0
    }

    fn dispatch(self) -> i32 {
        1
    }
}

pub fn serve(state: Arc<Mutex<App>>, key: string) -> i32 {
    match state.lock() {
        Ok(mut app) => app.handle_http(key),
        Err(_) => 500,
    }
}
"#;

fn owned_or_shared_self(rs: &str, name: &str) -> bool {
    rs.lines().any(|line| {
        let t = line.trim();
        let prefix = format!("fn {name}(");
        let pub_prefix = format!("pub fn {name}(");
        if !t.contains(&prefix) && !t.contains(&pub_prefix) {
            return false;
        }
        t.contains("fn ")
            && (t.contains("(self") || t.contains("(mut self") || t.contains("(&self"))
            && !t.contains("&mut self")
    })
}

#[test]
fn handle_method_must_mut_self_for_check_rate() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.522 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.522 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("self.clone().check_rate"),
        "P3.522 RED: must not clone self to call check_rate:\n{rs}"
    );
    assert!(
        !owned_or_shared_self(rs, "handle_method"),
        "P3.522 RED: handle_method must be &mut self (config read + &mut check_rate):\n{rs}"
    );
    assert!(
        rs.contains("fn handle_method(&mut self"),
        "P3.522 RED: expected &mut self handle_method:\n{rs}"
    );
    assert!(
        !owned_or_shared_self(rs, "handle_http"),
        "P3.522 RED: handle_http must be &mut self (MutexGuard cannot move App):\n{rs}"
    );
    assert!(
        rs.contains("fn handle_http(&mut self"),
        "P3.522 RED: expected &mut self handle_http:\n{rs}"
    );
    test.cargo_check().expect("P3.522 cargo-check");
}
