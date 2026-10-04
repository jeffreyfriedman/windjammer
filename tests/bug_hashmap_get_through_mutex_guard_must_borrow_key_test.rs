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
    feature = "integration_tests",
))]

//! FAILING REPRO — `HashMap::{get,contains_key}` through `MutexGuard` must borrow the key.
//!
//! Ecosystem `wj-sync` `SharedMap`:
//! ```ignore
//! match m.inner.lock() {
//!     Ok(g) => g.data.contains_key(key),  // or g.data.get(key)
//!     …
//! }
//! ```
//! Tip emits `contains_key(key.to_string())` / `get(key.to_string())` (E0308 expected `&_`).
//! Owned `HashMap` key methods (no mutex) already cargo-check.

#[path = "common/test_utils.rs"]
mod test_utils;

const SHARED_MAP_GET: &str = r#"
use std::collections::HashMap
use std::sync::{Arc, Mutex}

struct MapCell {
    data: HashMap<string, int>,
}

pub struct SharedMap {
    inner: Arc<Mutex<MapCell>>,
}

pub fn shared_map_new() -> SharedMap {
    SharedMap {
        inner: Arc::new(Mutex::new(MapCell {
            data: HashMap::new(),
        })),
    }
}

pub fn shared_map_get(m: SharedMap, key: string) -> (SharedMap, bool, int) {
    let result = match m.inner.lock() {
        Ok(g) => {
            match g.data.get(key) {
                Some(v) => (true, v),
                None => (false, 0),
            }
        }
        Err(_) => (false, 0),
    }
    (m, result.0, result.1)
}
"#;

const SHARED_MAP_CONTAINS: &str = r#"
use std::collections::HashMap
use std::sync::{Arc, Mutex}

struct MapCell {
    data: HashMap<string, int>,
}

pub struct SharedMap {
    inner: Arc<Mutex<MapCell>>,
}

pub fn shared_map_new() -> SharedMap {
    SharedMap {
        inner: Arc::new(Mutex::new(MapCell {
            data: HashMap::new(),
        })),
    }
}

pub fn shared_map_has(m: SharedMap, key: string) -> (SharedMap, bool) {
    let present = match m.inner.lock() {
        Ok(g) => g.data.contains_key(key),
        Err(_) => false,
    }
    (m, present)
}
"#;

#[test]
fn hashmap_get_through_mutex_guard_must_not_emit_key_to_string() {
    let generated = test_utils::compile_single(SHARED_MAP_GET);
    assert!(
        !generated.contains("get(key.to_string())")
            && !generated.contains(".get(key.to_string())"),
        "HashMap::get through MutexGuard must borrow key, not key.to_string():\n{generated}"
    );
}

#[test]
fn hashmap_contains_key_through_mutex_guard_must_not_emit_key_to_string() {
    let generated = test_utils::compile_single(SHARED_MAP_CONTAINS);
    assert!(
        !generated.contains("contains_key(key.to_string())"),
        "HashMap::contains_key through MutexGuard must borrow key:\n{generated}"
    );
}

#[test]
fn hashmap_get_through_mutex_guard_must_cargo_check() {
    test_utils::assert_stdlib_runtime_links(SHARED_MAP_GET, &[]);
}

#[test]
fn hashmap_contains_key_through_mutex_guard_must_cargo_check() {
    test_utils::assert_stdlib_runtime_links(SHARED_MAP_CONTAINS, &[]);
}
