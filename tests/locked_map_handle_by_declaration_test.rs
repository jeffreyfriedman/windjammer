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
//! A struct that wraps a lock-guarded map (`inner: Arc<Mutex<HashMap<K, V>>>`) and an alias of it are used through the
//! lock guard (`g.insert`, `g.get`) like any map, whatever the struct is called. Oracle: the generated crate builds.

use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

const SRC: &str = "\
use std::collections::HashMap
use std::sync::{Arc, Mutex}

pub struct Ledger<K, V> {
    inner: Arc<Mutex<HashMap<K, V>>>,
}

pub type LedgerSI = Ledger<string, int>

pub fn ledger_new() -> LedgerSI {
    Ledger { inner: Arc::new(Mutex::new(HashMap::new())) }
}

pub fn ledger_put(m: LedgerSI, key: string, value: int) -> LedgerSI {
    match m.inner.lock() {
        Ok(mut g) => {
            g.insert(key, value)
        }
        Err(_) => {}
    }
    m
}

pub fn ledger_get(m: LedgerSI, key: string) -> (LedgerSI, bool, int) {
    let result = match m.inner.lock() {
        Ok(g) => {
            match g.get(key) {
                Some(v) => (true, *v),
                None => (false, 0),
            }
        }
        Err(_) => (false, 0),
    }
    (m, result.0, result.1)
}

pub fn ledger_has(m: LedgerSI, key: string) -> bool {
    match m.inner.lock() {
        Ok(g) => g.contains_key(key),
        Err(_) => false,
    }
}
";

#[test]
fn locked_map_wrapper_is_a_map_by_its_declaration() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("ledger.wj");
    std::fs::write(&file, SRC).unwrap();
    let out = dir.path().join("out");
    // `std::sync` lowers to the runtime, so the generated crate is built with cargo.
    let b = Command::new(WJ).args(["build", "--no-lint", "-o"]).arg(&out).arg(&file).output().unwrap();
    let rs = std::fs::read_to_string(out.join("ledger.rs")).unwrap_or_default();
    assert!(
        b.status.success(),
        "the generated crate does not build:\n{}\n{}\n--- generated\n{rs}",
        String::from_utf8_lossy(&b.stdout),
        String::from_utf8_lossy(&b.stderr)
    );
}
