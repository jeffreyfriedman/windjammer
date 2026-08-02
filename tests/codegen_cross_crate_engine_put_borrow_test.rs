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

//! Cross-crate MemoryEngine::put(&Key) must not emit key.clone() at TxnManager call sites.
//! Mirrors external dogfood crates: owned Value consume + borrowed Key + take_value forward.

use std::fs;
use std::process::Command;

#[test]
fn cross_crate_memory_engine_put_borrows_key_no_clone() {
    let tmp = tempfile::tempdir().unwrap();
    let eng_src = tmp.path().join("eng_src");
    let eng_gen = tmp.path().join("eng_gen");
    fs::create_dir_all(eng_src.join("engine")).unwrap();
    fs::write(
        eng_src.join("engine/mod.wj"),
        "pub mod memory\npub mod storage\n",
    )
    .unwrap();
    fs::write(
        eng_src.join("engine/storage.wj"),
        r#"
pub struct Key { pub bytes: Vec<u8> }
pub enum Value {
    Null,
    Int64(i64),
    Text(string),
}

trait StorageEngine {
    fn put(self, key: Key, value: Value)
    fn delete(self, key: Key)
    fn get(self, key: Key) -> Option<Value>
}
"#,
    )
    .unwrap();
    fs::write(
        eng_src.join("engine/memory.wj"),
        r#"
use crate::engine::storage::{Key, Value}

fn value_i64(value: Value) -> i64 {
    match value {
        Value::Int64(v) => v,
        _ => 0,
    }
}

fn copy_key_bytes(key: Key) -> Vec<u8> {
    let mut out = Vec::new()
    let mut i = 0
    while i < key.bytes.len() {
        out.push(key.bytes[i])
        i = i + 1
    }
    out
}

pub struct MemoryEngine {
    handle: u64,
}

impl MemoryEngine {
    pub fn new() -> MemoryEngine {
        MemoryEngine { handle: 0 }
    }

    pub fn get(self, key: Key) -> Option<Value> {
        let _ = key.bytes
        None
    }

    pub fn put(self, key: Key, value: Value) {
        let v = value_i64(value)
        let bytes = copy_key_bytes(key)
        let _ = (self.handle, bytes.len(), v)
    }

    pub fn delete(self, key: Key) {
        let bytes = copy_key_bytes(key)
        let _ = (self.handle, bytes.len())
    }

    pub fn seed_write(self, key: Key, value: Value, version: u64) {
        let v = value_i64(value)
        let _ = (key.bytes.len(), v, version)
    }
}
"#,
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            eng_src.to_str().unwrap(),
            "--output",
            eng_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "eng build: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let meta = fs::read_to_string(eng_gen.join("metadata.json")).expect("metadata.json");
    assert!(
        meta.contains("MemoryEngine::put"),
        "metadata missing put: {meta}"
    );

    let txn_src = tmp.path().join("txn_src");
    let txn_gen = tmp.path().join("txn_gen");
    fs::create_dir_all(&txn_src).unwrap();
    fs::write(
        txn_src.join("manager.wj"),
        r#"
use engine::engine::memory::MemoryEngine
use engine::engine::storage::{Key, Value}

fn take_value(value: Value) -> Value {
    match value {
        Value::Null => Value::Null,
        Value::Int64(v) => Value::Int64(v),
        Value::Text(s) => Value::Text(s),
    }
}

pub struct TxnManager {
    engine: MemoryEngine,
}

impl TxnManager {
    pub fn new() -> TxnManager {
        TxnManager { engine: MemoryEngine::new() }
    }

    pub fn get(self, key: Key) -> Option<Value> {
        self.engine.get(key)
    }

    pub fn put(self, key: Key, value: Value) {
        self.engine.put(key, take_value(value))
    }

    pub fn delete(self, key: Key) {
        self.engine.delete(key)
    }

    pub fn seed_write(self, key: Key, value: Value, version: u64) {
        self.engine.seed_write(key, take_value(value), version)
    }
}
"#,
    )
    .unwrap();
    let metadata = format!("engine={}", eng_gen.join("metadata.json").display());
    let out = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            txn_src.join("manager.wj").to_str().unwrap(),
            "--output",
            txn_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &metadata,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "txn build: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rs = fs::read_to_string(txn_gen.join("manager.rs")).expect("manager.rs");
    eprintln!("GEN:\n{rs}");
    for line in rs.lines() {
        if line.contains("engine.put(") || line.contains("engine.delete(") {
            assert!(
                !line.contains("key.clone()"),
                "must not clone into &Key formal. Line: {line}\nFull:\n{rs}"
            );
        }
    }
}
