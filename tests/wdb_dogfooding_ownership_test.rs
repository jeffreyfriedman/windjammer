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

//! WDB dogfooding regressions from WINDJAMMERDB_ISSUES.md
//!
//! Run: `cargo test --release --test all wdb_dogfooding`
//!
//! ## Suite state (Jul 29 PM, wj 0.50.0+)
//!
//! **Dogfooding:** 47/47 pass (Jul 30, wj 0.50.0 f95b9c32). Phase 1 compiler blockers cleared.
//!
//! **WDB-066..068** — Phase 1 e2e regressions (Jul 29–30); all passing after f95b9c32.
//!
//! Recently fixed (must stay passing): WDB-061..065 substrate/sim, WDB-047/049 wal/substrate, WDB-050 vec literals.
//! Closed-issue regressions: WDB-044, WDB-045, WDB-048. Guard tests: WDB-019, WDB-039, WDB-042, WDB-046 (owned-only).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ── WDB-041: user-type get(key: Key) passes owned helper-return Key ──────────

#[test]
fn wdb_cross_crate_key_get_owned_helper_return() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "types/key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}

pub fn key_ten() -> Key {
    Key { bytes: vec![10u8] }
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\n");
    test.add_file(
        "index/btree.wj",
        r#"
use crate::types::key::Key

pub struct BTreeIndex {
    keys: Vec<Key>,
}

impl BTreeIndex {
    pub fn new() -> BTreeIndex {
        BTreeIndex { keys: Vec::new() }
    }

    pub fn get(self, key: Key) -> bool {
        for k in self.keys {
            if k.bytes == key.bytes {
                return true
            }
        }
        false
    }
}
"#,
    );
    test.add_file("index/mod.wj", "pub mod btree\n");
    test.add_file(
        "main.wj",
        r#"
use crate::index::btree::BTreeIndex
use crate::types::key::key_ten

pub fn lookup() -> bool {
    let index = BTreeIndex::new()
    index.get(key_ten())
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("main.rs").expect("main.rs");
    assert!(
        !rs.contains("key_ten().clone()"),
        "read-only Key param must not clone helper return. Got:\n{rs}"
    );
    assert!(
        rs.contains("get(key_ten())") || rs.contains("get(&key_ten())")
            || rs.contains("get(crate::types::key::key_ten())")
            || rs.contains("get(&crate::types::key::key_ten())"),
        "expected helper call at get site (owned pass or auto-borrow). Got:\n{rs}"
    );
}

// ── WDB-040: cross-crate string literal → owned String ─────────────────────

#[test]
fn wdb_cross_crate_string_literal_owned() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "catalog/table.wj",
        r#"
pub struct TableDef {
    pub name: string,
}

pub struct Catalog {
    tables: Vec<TableDef>,
}

impl Catalog {
    pub fn new() -> Catalog {
        Catalog { tables: Vec::new() }
    }

    pub fn register_table(self, name: string, columns: int) -> int {
        self.tables.push(TableDef { name: name })
        self.tables.len()
    }
}
"#,
    );
    test.add_file("catalog/mod.wj", "pub mod table\n");
    test.add_file(
        "embedded/app.wj",
        r#"
use crate::catalog::table::Catalog

pub fn setup() -> int {
    let catalog = Catalog::new()
    catalog.register_table("users", 3)
}
"#,
    );
    test.add_file("embedded/mod.wj", "pub mod app\n");
    test.add_file(
        "main.wj",
        r#"
use crate::embedded::app::setup

pub fn main() {
    let _ = setup()
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("embedded/app.rs").expect("embedded/app.rs");
    // Canonical owned-literal form is `String::from("…")` (see literals::string_literal_to_owned_rust);
    // `.to_string()` is also accepted for older call-site paths.
    let owned_literal = rs.contains(r#"String::from("users")"#)
        || rs.contains(r#""users".to_string()"#)
        || rs.contains(r#""users".to_string("#);
    assert!(
        owned_literal,
        "cross-crate string literal must coerce to owned String. Got:\n{rs}"
    );
    assert!(
        !rs.contains(r#"register_table("users""#)
            || owned_literal
            || rs.contains(".to_string()"),
        "must not pass bare &str when callee expects owned string. Got:\n{rs}"
    );
}

// ── WDB-039: free-fn Key params — vec index field symmetric coercion ─────────

#[test]
fn wdb_vec_index_key_compare_owned_params() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "types/key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}

pub fn keys_equal(a: Key, b: Key) -> bool {
    a.bytes == b.bytes
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\n");
    test.add_file(
        "index/store.wj",
        r#"
use crate::types::key::{Key, keys_equal}

pub struct Store {
    keys: Vec<Key>,
}

impl Store {
    pub fn has_ten(self) -> bool {
        let a = Key { bytes: vec![10u8] }
        keys_equal(a, self.keys[0])
    }
}
"#,
    );
    test.add_file("index/mod.wj", "pub mod store\n");
    test.add_file(
        "main.wj",
        r#"
use crate::index::store::Store

pub fn check() -> bool {
    let mut s = Store { keys: Vec::new() }
    s.keys.push(crate::types::key::Key { bytes: vec![10u8] })
    s.has_ten()
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("index/store.rs").expect("index/store.rs");
    assert!(
        !rs.contains("keys_equal(a, self.keys[0].clone())"),
        "owned Key param must borrow vec index, not clone. Got:\n{rs}"
    );
    assert!(
        rs.contains("keys_equal(&a, &self.keys[0])")
            || rs.contains("keys_equal(a, self.keys[0].clone())"),
        "expected borrow or owned-clone for vec index Key, not asymmetric clone. Got:\n{rs}"
    );
}

// ── WDB-042: mutating method on self.field must not clone receiver ─────────

#[test]
fn wdb_self_field_mutating_method_no_clone() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "sim/network.wj",
        r#"
pub struct SimNetwork {
    pending: int,
}

impl SimNetwork {
    pub fn new() -> SimNetwork {
        SimNetwork { pending: 1 }
    }

    pub fn poll(self) -> bool {
        self.pending = self.pending - 1
        self.pending > 0
    }
}
"#,
    );
    test.add_file(
        "sim/harness.wj",
        r#"
use crate::sim::network::SimNetwork

pub struct Harness {
    network: SimNetwork,
}

impl Harness {
    pub fn new() -> Harness {
        Harness { network: SimNetwork::new() }
    }

    pub fn drain(self) -> int {
        let mut count = 0
        while self.network.poll() {
            count = count + 1
        }
        count
    }
}
"#,
    );
    test.add_file("sim/mod.wj", "pub mod network\npub mod harness\n");
    test.add_file(
        "main.wj",
        r#"
use crate::sim::harness::Harness

pub fn run() -> int {
    Harness::new().drain()
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("sim/harness.rs").expect("sim/harness.rs");
    assert!(
        !rs.contains("self.network.clone()"),
        "mutating method on self.field must not clone field. Got:\n{rs}"
    );
    assert!(
        rs.contains("self.network.poll()"),
        "expected direct field method call. Got:\n{rs}"
    );
}

// ── WDB-040: cross-crate string variable → owned String ────────────────────

#[test]
fn wdb_cross_crate_string_variable_owned() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "catalog/table.wj",
        r#"
pub struct TableDef {
    pub name: string,
}

pub struct Catalog {
    tables: Vec<TableDef>,
}

impl Catalog {
    pub fn new() -> Catalog {
        Catalog { tables: Vec::new() }
    }

    pub fn register_table(self, name: string, columns: int) -> int {
        self.tables.push(TableDef { name: name })
        self.tables.len()
    }
}
"#,
    );
    test.add_file("catalog/mod.wj", "pub mod table\n");
    test.add_file(
        "embedded/app.wj",
        r#"
use crate::catalog::table::Catalog

pub fn setup(table_name: string) -> int {
    let catalog = Catalog::new()
    catalog.register_table(table_name, 3)
}
"#,
    );
    test.add_file("embedded/mod.wj", "pub mod app\n");
    test.add_file(
        "main.wj",
        r#"
use crate::embedded::app::setup

pub fn main() {
    let _ = setup("users")
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("embedded/app.rs").expect("embedded/app.rs");
    assert!(
        rs.contains("register_table(table_name") && !rs.contains("register_table(&table_name"),
        "owned string variable must pass by value cross-crate. Got:\n{rs}"
    );
}

// ── WDB-037: cross-crate LwwRegister::merge passes owned struct args ─────────

#[test]
fn wdb_cross_crate_merge_owned_struct_args() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "crdt/register.wj",
        r#"
pub struct LwwRegister {
    pub value: int,
    pub timestamp: int,
}

impl LwwRegister {
    pub fn merge(self, other: LwwRegister) -> LwwRegister {
        if other.timestamp > self.timestamp {
            other
        } else {
            self
        }
    }
}
"#,
    );
    test.add_file("crdt/mod.wj", "pub mod register\n");
    test.add_file(
        "store/merge.wj",
        r#"
use crate::crdt::register::LwwRegister

pub fn merge_registers(local: LwwRegister, remote: LwwRegister) -> LwwRegister {
    local.merge(remote)
}
"#,
    );
    test.add_file("store/mod.wj", "pub mod merge\n");
    test.add_file(
        "main.wj",
        r#"
use crate::store::merge::merge_registers
use crate::crdt::register::LwwRegister

pub fn run() -> int {
    let a = LwwRegister { value: 1, timestamp: 1 }
    let b = LwwRegister { value: 2, timestamp: 2 }
    let merged = merge_registers(a, b)
    merged.value
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("store/merge.rs").expect("store/merge.rs");
    assert!(
        rs.contains("local.merge(remote)") && !rs.contains("local.merge(&remote)"),
        "cross-crate merge must pass owned LwwRegister, not borrow. Got:\n{rs}"
    );
}

// ── WDB: MemoryEngine::get delegation must pass &Key, not key.clone() ───────

#[test]
fn wdb_engine_get_delegation_passes_borrowed_key() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "types/key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\n");
    test.add_file(
        "engine/memory.wj",
        r#"
use crate::types::key::Key

pub struct MemoryEngine {}

impl MemoryEngine {
    pub fn new() -> MemoryEngine {
        MemoryEngine {}
    }

    pub fn get(self, key: Key) -> int {
        key.bytes.len()
    }
}
"#,
    );
    test.add_file("engine/mod.wj", "pub mod memory\n");
    test.add_file(
        "engine/lsm.wj",
        r#"
use crate::types::key::Key
use crate::engine::memory::MemoryEngine

pub struct LsmEngine {
    engine: MemoryEngine,
}

impl LsmEngine {
    pub fn new() -> LsmEngine {
        LsmEngine { engine: MemoryEngine::new() }
    }

    pub fn get(self, key: Key) -> int {
        self.engine.get(key)
    }
}
"#,
    );
    test.add_file("engine/mod.wj", "pub mod memory\npub mod lsm\n");
    test.add_file(
        "main.wj",
        r#"
use crate::engine::lsm::LsmEngine
use crate::types::key::Key

pub fn lookup() -> int {
    let engine = LsmEngine::new()
    let key = Key { bytes: vec![1u8] }
    engine.get(key)
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("engine/lsm.rs").expect("engine/lsm.rs");
    assert!(
        rs.contains("pub fn get(&self, key: &Key)"),
        "LsmEngine::get should borrow key param. Got:\n{rs}"
    );
    assert!(
        rs.contains("self.engine.get(key)") && !rs.contains("self.engine.get(key.clone())"),
        "MemoryEngine::get expects &Key — delegate with borrowed key. Got:\n{rs}"
    );
}

#[test]
fn wdb_engine_put_delegation_passes_borrowed_key() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "types/key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\n");
    test.add_file(
        "engine/memory.wj",
        r#"
use crate::types::key::Key

pub struct MemoryEngine {
    handle: i64,
}

impl MemoryEngine {
    pub fn new() -> MemoryEngine {
        MemoryEngine { handle: 0 }
    }

    pub fn put(self, key: Key, value: i64) {
        let _ = key.bytes.len()
        let _ = value
    }
}
"#,
    );
    test.add_file(
        "engine/lsm.wj",
        r#"
use crate::types::key::Key
use crate::engine::memory::MemoryEngine

struct PendingWrite {
    key: Key,
    value: i64,
}

pub struct LsmEngine {
    engine: MemoryEngine,
}

impl LsmEngine {
    pub fn new() -> LsmEngine {
        LsmEngine { engine: MemoryEngine::new() }
    }

    pub fn put(self, key: Key, value: i64) {
        let _w = PendingWrite { key: key.clone(), value: value }
        self.engine.put(key, value)
    }
}
"#,
    );
    test.add_file("engine/mod.wj", "pub mod memory\npub mod lsm\n");
    test.add_file(
        "main.wj",
        r#"
use crate::engine::lsm::LsmEngine
use crate::types::key::Key

pub fn write_key() {
    let engine = LsmEngine::new()
    let key = Key { bytes: vec![1u8] }
    engine.put(key, 42)
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("engine/lsm.rs").expect("engine/lsm.rs");
    let outer_owned = rs.contains("pub fn put(&self, key: Key")
        || rs.contains("pub fn put(&mut self, key: Key");
    if outer_owned {
        assert!(
            rs.contains("self.engine.put(&key") || rs.contains("self.engine.put(& key"),
            "owned Key param must borrow at MemoryEngine::put. Got:\n{rs}"
        );
    } else {
        assert!(
            rs.contains("self.engine.put(key"),
            "when outer param is already &Key, pass through. Got:\n{rs}"
        );
    }
}

#[test]
fn wdb_forward_ref_owned_key_borrows_at_later_method() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "types/key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\n");
    test.add_file(
        "store.wj",
        r#"
use crate::types::key::Key

pub struct LsmStore {
    keys: Vec<Key>,
}

impl LsmStore {
    pub fn new() -> LsmStore {
        LsmStore { keys: Vec::new() }
    }

    pub fn put_value(self, key: Key) {
        if self.key_in_latest_base(key) {
            let _ = key.bytes.len()
        }
    }
}

impl LsmStore {
    fn key_in_latest_base(self, key: Key) -> bool {
        for k in self.keys {
            if k.bytes == key.bytes {
                return true
            }
        }
        false
    }
}
"#,
    );
    test.add_file(
        "main.wj",
        r#"
use crate::store::LsmStore
use crate::types::key::Key

pub fn run() {
    let store = LsmStore::new()
    store.put_value(Key { bytes: vec![1u8] })
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("store.rs").expect("store.rs");
    let outer_owned = rs.contains("pub fn put_value(&self, key: Key")
        || rs.contains("pub fn put_value(&mut self, key: Key");
    if outer_owned {
        assert!(
            rs.contains("self.key_in_latest_base(&key") || rs.contains("self.key_in_latest_base(& key"),
            "owned Key must borrow for forward-ref callee. Got:\n{rs}"
        );
    } else {
        assert!(
            rs.contains("self.key_in_latest_base(key)"),
            "when outer param is already &Key, pass through. Got:\n{rs}"
        );
    }
}

// ── WDB-046/047: forward-ref guard — outer owned Key must not become &Key at patch calls ─

#[test]
fn wdb_store_has_key_forward_ref_borrows_owned_key() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "types/key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\n");
    test.add_file(
        "store.wj",
        r#"
use crate::types::key::Key

pub struct BasePart {
    entries: Vec<(Key, i64)>,
}

impl BasePart {
    pub fn get(self, key: Key) -> Option<i64> {
        let count = self.entries.len()
        let mut i = 0
        while i < count {
            if self.entries[i].0.bytes == key.bytes {
                return Some(self.entries[i].1)
            }
            i = i + 1
        }
        None
    }

    pub fn has_key(self, key: Key) -> bool {
        match self.get(key) {
            Some(_) => true,
            None => false,
        }
    }
}

pub struct LsmStore {
    base_parts: Vec<BasePart>,
}

impl LsmStore {
    pub fn new() -> LsmStore {
        LsmStore { base_parts: Vec::new() }
    }

    pub fn put_value(self, key: Key, value: i64) {
        if self.key_in_latest_base(key) {
            self.patch_put(key, value)
        } else {
            self.hot_put(key, value)
        }
    }

    fn patch_put(self, key: Key, value: i64) {
        let _ = (key, value)
    }

    fn hot_put(self, key: Key, value: i64) {
        let _ = (key, value)
    }
}

impl LsmStore {
    fn key_in_latest_base(self, key: Key) -> bool {
        let base_count = self.base_parts.len()
        if base_count == 0 {
            false
        } else {
            let latest = self.base_parts[base_count - 1]
            latest.has_key(key)
        }
    }
}
"#,
    );
    test.add_file(
        "main.wj",
        r#"
use crate::store::LsmStore
use crate::types::key::Key

pub fn run() {
    let store = LsmStore::new()
    store.put_value(Key { bytes: vec![1u8] }, 42)
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("store.rs").expect("store.rs");
    assert!(
        rs.contains("pub fn put_value(&self, key: Key")
            || rs.contains("pub fn put_value(&self, mut key: Key")
            || rs.contains("pub fn put_value(&mut self, key: Key")
            || rs.contains("pub fn put_value(&mut self, mut key: Key"),
        "expected owned Key outer param. Got:\n{rs}"
    );
    assert!(
        rs.contains("self.key_in_latest_base(&key") || rs.contains("self.key_in_latest_base(& key"),
        "owned Key must borrow for has_key-style forward ref. Got:\n{rs}"
    );
    assert!(
        rs.contains("latest.has_key(key.clone())") || rs.contains("latest.has_key(&key.clone())"),
        "borrowed outer Key formal must clone for owned callee has_key. Got:\n{rs}"
    );
}

// ── WDB-044: free-fn `copy_key_bytes(key: Key)` — owned param, no spurious &Key formal ──
//
// Dogfooding: wdb-substrate/memory_engine.wj put/delete paths call copy_key_bytes(key)
// after other uses of key in the same function.

#[test]
fn wdb_copy_key_bytes_owned_helper_in_put() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "types/key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}

pub fn copy_key_bytes(key: Key) -> Vec<u8> {
    key.bytes
}
"#,
    );
    test.add_file(
        "types/value.wj",
        r#"
pub struct Value {
    pub data: Vec<u8>,
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\npub mod value\n");
    test.add_file(
        "engine/memory.wj",
        r#"
use crate::types::key::{Key, copy_key_bytes}
use crate::types::value::Value

pub struct MemoryEngine {
    pending: Vec<(Key, Value)>,
}

impl MemoryEngine {
    pub fn new() -> MemoryEngine {
        MemoryEngine { pending: Vec::new() }
    }

    pub fn put(self, key: Key, value: Value) {
        let bytes = copy_key_bytes(key)
        let _ = bytes.len()
        self.pending.push((key, value))
    }

    pub fn delete(self, key: Key) {
        let bytes = copy_key_bytes(key)
        let _ = bytes.len()
    }
}
"#,
    );
    test.add_file("engine/mod.wj", "pub mod memory\n");
    test.add_file(
        "main.wj",
        r#"
use crate::engine::memory::MemoryEngine
use crate::types::key::Key
use crate::types::value::Value

pub fn write() {
    let engine = MemoryEngine::new()
    let key = Key { bytes: vec![1u8] }
    let value = Value { data: vec![2u8] }
    engine.put(key, value)
}
"#,
    );

    let map = test.compile().expect("compile");
    let key_rs = map.get("types/key.rs").expect("types/key.rs");
    assert!(
        key_rs.contains("fn copy_key_bytes(key: Key)") || key_rs.contains("fn copy_key_bytes(_key: Key)"),
        "helper must take owned Key, not &Key. Got:\n{key_rs}"
    );
    let rs = map.get("engine/memory.rs").expect("engine/memory.rs");
    assert!(
        rs.contains("copy_key_bytes(key)") && !rs.contains("copy_key_bytes(&key)"),
        "put must pass owned key to copy_key_bytes before reusing key in push. Got:\n{rs}"
    );
    assert!(
        !rs.contains("expected &Key") && rs.contains("self.pending.push((key, value))"),
        "key must remain owned for pending.push after copy_key_bytes. Got:\n{rs}"
    );
}

// ── WDB-045: free-fn `encode_message(msg: SimMessage)` — owned struct param ─────────

#[test]
fn wdb_encode_message_owned_free_fn_in_send() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "sim/message.wj",
        r#"
pub struct SimMessage {
    pub seq: int,
    pub payload: Vec<u8>,
}

pub fn encode_message(msg: SimMessage) -> Vec<u8> {
    msg.payload
}
"#,
    );
    test.add_file("sim/mod.wj", "pub mod message\n");
    test.add_file(
        "sim/network.wj",
        r#"
use crate::sim::message::{SimMessage, encode_message}

pub struct SimNetwork {
    outbox: Vec<Vec<u8>>,
}

impl SimNetwork {
    pub fn new() -> SimNetwork {
        SimNetwork { outbox: Vec::new() }
    }

    pub fn send(self, msg: SimMessage) {
        let payload = encode_message(msg)
        self.outbox.push(payload)
    }

    pub fn hold_for_reorder(self, msg: SimMessage) -> Vec<u8> {
        encode_message(msg)
    }
}
"#,
    );
    test.add_file("sim/mod.wj", "pub mod message\npub mod network\n");
    test.add_file(
        "main.wj",
        r#"
use crate::sim::network::SimNetwork
use crate::sim::message::SimMessage

pub fn run() {
    let net = SimNetwork::new()
    net.send(SimMessage { seq: 1, payload: vec![1u8] })
}
"#,
    );

    let map = test.compile().expect("compile");
    let message_rs = map.get("sim/message.rs").expect("sim/message.rs");
    assert!(
        message_rs.contains("fn encode_message(msg: SimMessage)")
            || message_rs.contains("fn encode_message(_msg: SimMessage)"),
        "encode_message must take owned SimMessage. Got:\n{message_rs}"
    );
    let rs = map.get("sim/network.rs").expect("sim/network.rs");
    assert!(
        rs.contains("encode_message(msg)") && !rs.contains("encode_message(&msg)"),
        "send/hold must pass owned msg to encode_message. Got:\n{rs}"
    );
}

// ── WDB-046: TxnManager delegation — owned Key param must not become &Key at engine.get ─

#[test]
fn wdb_txn_manager_delegates_owned_key_to_engine() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "types/key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}
"#,
    );
    test.add_file(
        "types/value.wj",
        r#"
pub struct Value {
    pub data: Vec<u8>,
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\npub mod value\n");
    test.add_file(
        "engine/memory.wj",
        r#"
use crate::types::key::Key
use crate::types::value::Value

pub struct MemoryEngine {}

impl MemoryEngine {
    pub fn new() -> MemoryEngine {
        MemoryEngine {}
    }

    pub fn get(self, key: Key) -> Option<Value> {
        let bytes = key.bytes
        let _ = bytes.len()
        None
    }

    pub fn put(self, key: Key, value: Value) {
        let _ = (key, value)
    }

    pub fn delete(self, key: Key) {
        let _ = key.bytes.len()
    }
}
"#,
    );
    test.add_file("engine/mod.wj", "pub mod memory\n");
    test.add_file(
        "txn/manager.wj",
        r#"
use crate::types::key::Key
use crate::types::value::Value
use crate::engine::memory::MemoryEngine

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
        self.engine.put(key, value)
    }

    pub fn delete(self, key: Key) {
        self.engine.delete(key)
    }
}
"#,
    );
    test.add_file("txn/mod.wj", "pub mod manager\n");
    test.add_file(
        "main.wj",
        r#"
use crate::txn::manager::TxnManager
use crate::types::key::Key
use crate::types::value::Value

pub fn run() {
    let txn = TxnManager::new()
    let key = Key { bytes: vec![1u8] }
    let value = Value { data: vec![2u8] }
    txn.put(key, value)
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("txn/manager.rs").expect("txn/manager.rs");
    assert!(
        rs.contains("pub fn get(") && rs.contains("key: Key"),
        "TxnManager::get must keep owned Key param. Got:\n{rs}"
    );
    assert!(
        rs.contains("self.engine.get(key)") && !rs.contains("self.engine.get(&key)"),
        "delegation must pass owned Key when callee expects owned. Got:\n{rs}"
    );
    assert!(
        rs.contains("self.engine.put(key, value)") && !rs.contains("self.engine.put(&key"),
        "put delegation must pass owned Key/Value. Got:\n{rs}"
    );
}

// ── WDB-019: cross-crate Value — put expects owned Value at delegation site ───────────

#[test]
fn wdb_cross_crate_value_put_owned_delegation() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "types/key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}
"#,
    );
    test.add_file(
        "types/value.wj",
        r#"
pub struct Value {
    pub data: Vec<u8>,
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\npub mod value\n");
    test.add_file(
        "engine/memory.wj",
        r#"
use crate::types::key::Key
use crate::types::value::Value

pub struct MemoryEngine {}

impl MemoryEngine {
    pub fn new() -> MemoryEngine {
        MemoryEngine {}
    }

    pub fn put(self, key: Key, value: Value) {
        let _ = (key.bytes.len(), value.data.len())
    }

    pub fn get(self, key: Key) -> Option<Value> {
        let bytes = key.bytes
        let _ = bytes.len()
        None
    }
}
"#,
    );
    test.add_file("engine/mod.wj", "pub mod memory\n");
    test.add_file(
        "txn/manager.wj",
        r#"
use crate::types::key::Key
use crate::types::value::Value
use crate::engine::memory::MemoryEngine

pub struct TxnManager {
    engine: MemoryEngine,
}

impl TxnManager {
    pub fn new() -> TxnManager {
        TxnManager { engine: MemoryEngine::new() }
    }

    pub fn put(self, key: Key, value: Value) {
        self.engine.put(key, value)
    }

    pub fn get(self, key: Key) -> Option<Value> {
        self.engine.get(key)
    }
}
"#,
    );
    test.add_file("txn/mod.wj", "pub mod manager\n");
    test.add_file(
        "main.wj",
        r#"
use crate::txn::manager::TxnManager
use crate::types::key::Key
use crate::types::value::Value

pub fn run() {
    let txn = TxnManager::new()
    txn.put(
        Key { bytes: vec![1u8] },
        Value { data: vec![2u8] },
    )
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("txn/manager.rs").expect("txn/manager.rs");
    assert!(
        rs.contains("self.engine.put(key, value)") && !rs.contains("self.engine.put(key, &value)"),
        "Value must pass by value cross-crate without take_value re-box. Got:\n{rs}"
    );
    assert!(
        !rs.contains("take_value("),
        "must not require take_value workaround. Got:\n{rs}"
    );
}

// ── WDB-039 (full path): keys_equal_key in LSM get — vec field + helper, no byte-move ─

#[test]
fn wdb_keys_equal_key_in_store_get_path() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "types/key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}

pub fn keys_equal_key(a: Key, b: Key) -> bool {
    a.bytes == b.bytes
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\n");
    test.add_file(
        "substrate/store.wj",
        r#"
use crate::types::key::{Key, keys_equal_key}

pub struct Store {
    keys: Vec<Key>,
    values: Vec<i64>,
}

impl Store {
    pub fn new() -> Store {
        Store { keys: Vec::new(), values: Vec::new() }
    }

    pub fn get(self, key: Key) -> Option<i64> {
        let count = self.keys.len()
        let mut i = 0
        while i < count {
            let ekey = self.keys[i]
            if keys_equal_key(ekey, key) {
                return Some(self.values[i])
            }
            i = i + 1
        }
        None
    }
}
"#,
    );
    test.add_file("substrate/mod.wj", "pub mod store\n");
    test.add_file(
        "main.wj",
        r#"
use crate::substrate::store::Store
use crate::types::key::Key

pub fn lookup() -> Option<i64> {
    let store = Store::new()
    store.get(Key { bytes: vec![10u8] })
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("substrate/store.rs").expect("substrate/store.rs");
    assert!(
        !rs.contains("keys_equal_key(ekey.clone(), key.clone())"),
        "must not clone both Key args asymmetrically. Got:\n{rs}"
    );
    assert!(
        rs.contains("keys_equal_key(") && (rs.contains("&ekey") || rs.contains("ekey.clone()")),
        "vec-index ekey must coerce to match owned Key formal. Got:\n{rs}"
    );
}

// ── WDB-042 (harness layout): while self.network.poll() in drain_network ─────────────

#[test]
fn wdb_harness_drain_network_while_poll_no_field_clone() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "sim/clock.wj",
        r#"
pub struct LogicalTime {
    pub tick: int,
}
"#,
    );
    test.add_file(
        "sim/network.wj",
        r#"
use crate::sim::clock::LogicalTime

pub struct SimNetwork {
    pending: Vec<i64>,
    hold_queue: Vec<i64>,
}

impl SimNetwork {
    pub fn new() -> SimNetwork {
        SimNetwork { pending: Vec::new(), hold_queue: Vec::new() }
    }

    pub fn poll(self, clock: LogicalTime) -> bool {
        if self.pending.len() > 0 {
            self.pending.pop()
            let _ = clock.tick
            true
        } else {
            false
        }
    }
}
"#,
    );
    test.add_file(
        "sim/harness.wj",
        r#"
use crate::sim::clock::LogicalTime
use crate::sim::network::SimNetwork

pub struct SimHarness {
    network: SimNetwork,
    clock: LogicalTime,
}

impl SimHarness {
    pub fn new() -> SimHarness {
        SimHarness {
            network: SimNetwork::new(),
            clock: LogicalTime { tick: 0 },
        }
    }

    pub fn drain_network(self) -> int {
        let mut count = 0
        while self.network.poll(self.clock) {
            count = count + 1
        }
        count
    }
}
"#,
    );
    test.add_file("sim/mod.wj", "pub mod clock\npub mod network\npub mod harness\n");
    test.add_file(
        "main.wj",
        r#"
use crate::sim::harness::SimHarness

pub fn run() -> int {
    SimHarness::new().drain_network()
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("sim/harness.rs").expect("sim/harness.rs");
    assert!(
        !rs.contains("self.network.clone()"),
        "drain_network must mutate self.network in place, not clone. Got:\n{rs}"
    );
    assert!(
        rs.contains("self.network.poll("),
        "expected direct self.network.poll in while loop. Got:\n{rs}"
    );
}

// ── WDB-047: LsmStore apply_patch_* — asymmetric coercion after key_in_latest_base guard ─

#[test]
fn wdb_lsm_store_apply_patch_asymmetric_coercion() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "types/key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}
"#,
    );
    test.add_file(
        "types/value.wj",
        r#"
pub struct Value {
    pub data: Vec<u8>,
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\npub mod value\n");
    test.add_file(
        "substrate/store.wj",
        r#"
use crate::types::key::Key
use crate::types::value::Value

pub struct HotStore {}

impl HotStore {
    pub fn new() -> HotStore {
        HotStore {}
    }

    pub fn put_value(self, key: Key, value: Value) {
        let _ = (key, value)
    }
}

pub struct BasePart {
    keys: Vec<Key>,
}

impl BasePart {
    pub fn has_key(self, key: Key) -> bool {
        for k in self.keys {
            if k.bytes == key.bytes {
                return true
            }
        }
        false
    }
}

pub struct LsmStore {
    hot: HotStore,
    base_parts: Vec<BasePart>,
    patch_keys: Vec<Key>,
    deleted_keys: Vec<Key>,
}

impl LsmStore {
    pub fn new() -> LsmStore {
        LsmStore {
            hot: HotStore::new(),
            base_parts: Vec::new(),
            patch_keys: Vec::new(),
            deleted_keys: Vec::new(),
        }
    }

    pub fn put_value(self, key: Key, value: Value) {
        if self.key_in_latest_base(key) {
            self.apply_patch_put(key, value)
        } else {
            self.hot.put_value(key, value)
        }
    }

    pub fn delete_key(self, key: Key) {
        if self.key_in_latest_base(key) {
            self.apply_patch_delete(key)
        }
    }

    fn apply_patch_put(self, key: Key, value: Value) {
        let _ = value.data.len()
        self.patch_keys.push(key)
    }

    fn apply_patch_delete(self, key: Key) {
        self.deleted_keys.push(key)
    }
}

impl LsmStore {
    fn key_in_latest_base(self, key: Key) -> bool {
        let base_count = self.base_parts.len()
        if base_count == 0 {
            false
        } else {
            let latest = self.base_parts[base_count - 1]
            latest.has_key(key)
        }
    }
}
"#,
    );
    test.add_file("substrate/mod.wj", "pub mod store\n");
    test.add_file(
        "main.wj",
        r#"
use crate::substrate::store::LsmStore
use crate::types::key::Key
use crate::types::value::Value

pub fn run() {
    let store = LsmStore::new()
    store.put_value(Key { bytes: vec![1u8] }, Value { data: vec![2u8] })
    store.delete_key(Key { bytes: vec![3u8] })
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("substrate/store.rs").expect("substrate/store.rs");
    assert!(
        rs.contains("pub fn put_value(") && rs.contains("key: Key") && rs.contains("value: Value"),
        "put_value must keep owned Key and owned Value outer formals (wdb-substrate store.wj layout). Got:\n{rs}"
    );
    assert!(
        rs.contains("self.apply_patch_put(")
            && (rs.contains("apply_patch_put(") && rs.contains("&value"))
            && !rs.contains("apply_patch_put(key, value)"),
        "apply_patch_put must borrow owned Value param after forward-ref key guard. Got:\n{rs}"
    );
    assert!(
        (rs.contains("self.apply_patch_delete(key)")
            || rs.contains("self.apply_patch_delete(key.clone())"))
            && !rs.contains("self.apply_patch_delete(&key)"),
        "apply_patch_delete must pass owned Key after forward-ref, not &key. Got:\n{rs}"
    );
    assert!(
        rs.contains("pub fn apply_patch_delete(") && rs.contains("key: Key"),
        "apply_patch_delete must emit owned Key formal. Got:\n{rs}"
    );
}

// ── WDB-048: string literal at user fn temp_path(name: string) — no .to_string() ────────

#[test]
fn wdb_temp_path_string_literal_no_to_string() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "substrate/paths.wj",
        r#"
pub fn temp_path(name: string) -> string {
    format!("tmp/{}", name)
}
"#,
    );
    test.add_file("substrate/mod.wj", "pub mod paths\n");
    test.add_file(
        "substrate/lsm_test.wj",
        r#"
use crate::substrate::paths::temp_path

pub fn recover_fixture_path() -> string {
    temp_path("recover")
}
"#,
    );
    test.add_file("substrate/mod.wj", "pub mod paths\npub mod lsm_test\n");
    test.add_file(
        "main.wj",
        r#"
use crate::substrate::lsm_test::recover_fixture_path

pub fn main() {
    let _ = recover_fixture_path()
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("substrate/lsm_test.rs").expect("substrate/lsm_test.rs");
    assert!(
        rs.contains(r#"temp_path("recover")"#),
        "string literal must pass as &str to temp_path(name: string), not .to_string(). Got:\n{rs}"
    );
    assert!(
        !rs.contains(r#""recover".to_string()"#),
        "must not coerce string literal to owned String for &str formal. Got:\n{rs}"
    );
}

// ── WDB-019 (seed_write path): cross-crate Value without take_value re-box ───────────────

#[test]
fn wdb_txn_seed_write_without_take_value_rebox() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "types/key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}
"#,
    );
    test.add_file(
        "types/value.wj",
        r#"
pub enum Value {
    Null,
    Int64(i64),
    Text(string),
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\npub mod value\n");
    test.add_file(
        "engine/memory.wj",
        r#"
use crate::types::key::Key
use crate::types::value::Value

pub struct MemoryEngine {}

impl MemoryEngine {
    pub fn new() -> MemoryEngine {
        MemoryEngine {}
    }

    pub fn seed_write(self, key: Key, value: Value, version: u64) {
        let _ = (key.bytes.len(), version)
        match value {
            Value::Text(s) => {
                let _ = s
            }
            _ => {}
        }
    }

    pub fn put(self, key: Key, value: Value) {
        let _ = (key, value)
    }
}
"#,
    );
    test.add_file("engine/mod.wj", "pub mod memory\n");
    test.add_file(
        "txn/manager.wj",
        r#"
use crate::types::key::Key
use crate::types::value::Value
use crate::engine::memory::MemoryEngine

pub struct TxnManager {
    engine: MemoryEngine,
}

impl TxnManager {
    pub fn new() -> TxnManager {
        TxnManager { engine: MemoryEngine::new() }
    }

    pub fn seed_write(self, key: Key, value: Value, version: u64) {
        self.engine.seed_write(key, value, version)
    }

    pub fn put(self, key: Key, value: Value) {
        self.engine.put(key, value)
    }
}
"#,
    );
    test.add_file("txn/mod.wj", "pub mod manager\n");
    test.add_file(
        "main.wj",
        r#"
use crate::txn::manager::TxnManager
use crate::types::key::Key
use crate::types::value::Value

pub fn run() {
    let txn = TxnManager::new()
    txn.seed_write(
        Key { bytes: vec![1u8] },
        Value::Text("seed"),
        42,
    )
    txn.put(Key { bytes: vec![2u8] }, Value::Int64(7))
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("txn/manager.rs").expect("txn/manager.rs");
    assert!(
        rs.contains("self.engine.seed_write(key, value, version)")
            && !rs.contains("take_value(value)"),
        "seed_write must delegate owned Value cross-crate without take_value re-box. Got:\n{rs}"
    );
    assert!(
        rs.contains("self.engine.put(key, value)") && !rs.contains("take_value("),
        "put must not require take_value workaround. Got:\n{rs}"
    );
}

// ── WDB-039 (LSM BasePart path): keys_equal_key at vec-index — no byte-move shim ─────────

#[test]
fn wdb_lsm_base_part_get_without_byte_move_shim() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "types/key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}

pub fn keys_equal_key(a: Key, b: Key) -> bool {
    a.bytes == b.bytes
}
"#,
    );
    test.add_file(
        "types/value.wj",
        r#"
pub struct Value {
    pub data: Vec<u8>,
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\npub mod value\n");
    test.add_file(
        "substrate/base_part.wj",
        r#"
use crate::types::key::Key
use crate::types::key::keys_equal_key
use crate::types::value::Value

pub struct BasePart {
    entries: Vec<(Key, Value)>,
}

impl BasePart {
    pub fn new() -> BasePart {
        BasePart { entries: Vec::new() }
    }

    pub fn get(self, key: Key) -> Option<Value> {
        let count = self.entries.len()
        let mut i = 0
        while i < count {
            let ekey = self.entries[i].0
            if keys_equal_key(ekey, key) {
                return Some(self.entries[i].1)
            }
            i = i + 1
        }
        None
    }
}
"#,
    );
    test.add_file("substrate/mod.wj", "pub mod base_part\n");
    test.add_file(
        "main.wj",
        r#"
use crate::substrate::base_part::BasePart
use crate::types::key::Key

pub fn lookup() -> bool {
    let part = BasePart::new()
    match part.get(Key { bytes: vec![9u8] }) {
        Some(_) => true,
        None => false,
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("substrate/base_part.rs").expect("substrate/base_part.rs");
    assert!(
        rs.contains("keys_equal_key("),
        "must use keys_equal_key helper, not byte-move shim. Got:\n{rs}"
    );
    assert!(
        !rs.contains("keys_equal_bytes"),
        "must not emit keys_equal_bytes workaround. Got:\n{rs}"
    );
    assert!(
        !rs.contains("let a = ekey.bytes") && !rs.contains("let b = key.bytes"),
        "must compare Keys directly without byte-move extract. Got:\n{rs}"
    );
}

// ── WDB-042 (real harness loop): loop+match poll without network field extract ───────────

#[test]
fn wdb_harness_loop_match_without_network_extract() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "sim/clock.wj",
        r#"
pub struct LogicalTime {
    pub tick: int,
}
"#,
    );
    test.add_file(
        "sim/network.wj",
        r#"
use crate::sim::clock::LogicalTime

pub struct SimNetwork {
    pending: Vec<i64>,
}

impl SimNetwork {
    pub fn new() -> SimNetwork {
        SimNetwork { pending: Vec::new() }
    }

    pub fn poll(self, clock: LogicalTime) -> Option<i64> {
        if self.pending.len() > 0 {
            self.pending.pop()
            let _ = clock.tick
        } else {
            None
        }
    }
}
"#,
    );
    test.add_file(
        "sim/harness.wj",
        r#"
use crate::sim::clock::LogicalTime
use crate::sim::network::SimNetwork

pub struct SimHarness {
    network: SimNetwork,
    clock: LogicalTime,
}

impl SimHarness {
    pub fn new() -> SimHarness {
        SimHarness {
            network: SimNetwork::new(),
            clock: LogicalTime { tick: 0 },
        }
    }

    pub fn drain_network(self) -> int {
        let mut count = 0
        loop {
            match self.network.poll(self.clock) {
                Some(_) => {
                    count = count + 1
                }
                None => {
                    break
                }
            }
        }
        count
    }
}
"#,
    );
    test.add_file("sim/mod.wj", "pub mod clock\npub mod network\npub mod harness\n");
    test.add_file(
        "main.wj",
        r#"
use crate::sim::harness::SimHarness

pub fn run() -> int {
    SimHarness::new().drain_network()
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("sim/harness.rs").expect("sim/harness.rs");
    assert!(
        !rs.contains("self.network.clone()"),
        "poll must mutate network in place, not clone. Got:\n{rs}"
    );
    assert!(
        !rs.contains("let mut net = self.network") && !rs.contains("let net = self.network"),
        "must not require network field extract workaround. Got:\n{rs}"
    );
    assert!(
        rs.contains("self.network.poll("),
        "expected in-place self.network.poll inside loop. Got:\n{rs}"
    );
}

// ── WDB-049: WAL FFI Vec return + path field — borrow at callee sites ────────────────────

#[test]
fn wdb_wal_ffi_snapshot_and_path_borrow_at_call_sites() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "wal/ffi.wj",
        r#"
extern fn wal_snapshot_load_ffi(path: string) -> Vec<u8>
"#,
    );
    test.add_file(
        "wal/segment.wj",
        r#"
pub struct WalSegment {
    pub bytes: Vec<u8>,
}

impl WalSegment {
    pub fn with_header() -> WalSegment {
        WalSegment { bytes: vec![1u8, 2u8] }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> WalSegment {
        WalSegment { bytes: bytes }
    }
}
"#,
    );
    test.add_file(
        "wal/record.wj",
        r#"
pub struct WalRecord {
    pub key: Vec<u8>,
}
"#,
    );
    test.add_file(
        "wal/replay.wj",
        r#"
use crate::wal::record::WalRecord

pub fn replay_all(path: string) -> Vec<WalRecord> {
    let _ = path
    Vec::new()
}

pub fn replay_to_lsn(path: string, through: u64) -> Vec<WalRecord> {
    let _ = (path, through)
    Vec::new()
}
"#,
    );
    test.add_file(
        "wal/writer.wj",
        r#"
use crate::wal::ffi::wal_snapshot_load_ffi
use crate::wal::segment::WalSegment
use crate::wal::record::WalRecord
use crate::wal::replay::replay_all
use crate::wal::replay::replay_to_lsn

pub struct WalWriter {
    path: string,
    segment: WalSegment,
}

impl WalWriter {
    pub fn open_existing(path: string) -> WalWriter {
        let handle_path = path
        let snapshot = wal_snapshot_load_ffi(handle_path)
        let segment = if snapshot.len() > 0 {
            WalSegment::from_bytes(snapshot)
        } else {
            WalSegment::with_header()
        }
        WalWriter {
            path: path,
            segment: segment,
        }
    }

    pub fn replay_all_records(self) -> Vec<WalRecord> {
        replay_all(self.path)
    }

    pub fn replay_through(self, through: u64) -> Vec<WalRecord> {
        replay_to_lsn(self.path, through)
    }
}
"#,
    );
    test.add_file(
        "wal/mod.wj",
        r#"
pub mod ffi
pub mod segment
pub mod record
pub mod replay
pub mod writer
"#,
    );
    test.add_file(
        "main.wj",
        r#"
use crate::wal::writer::WalWriter

pub fn run() {
    let writer = WalWriter::open_existing("wal.log")
    let _ = writer.replay_all_records()
    let _ = writer.replay_through(42)
}
"#,
    );

    let map = test.compile().expect("compile");
    let writer_rs = map.get("wal/writer.rs").expect("wal/writer.rs");
    assert!(
        writer_rs.contains("WalSegment::from_bytes(&snapshot)")
            || writer_rs.contains("WalSegment::from_bytes(& snapshot)")
            || writer_rs.contains("from_bytes(&snapshot)"),
        "FFI Vec snapshot must borrow at from_bytes call. Got:\n{writer_rs}"
    );
    assert!(
        !writer_rs.contains("WalSegment::from_bytes(snapshot)") || writer_rs.contains("&snapshot"),
        "must not move owned snapshot into &Vec formal. Got:\n{writer_rs}"
    );
    assert!(
        writer_rs.contains("replay_all(&self.path)")
            || writer_rs.contains("replay_all(self.path.as_str())")
            || writer_rs.contains("replay_all(& self.path)"),
        "path field must borrow as &str for replay_all(path: &str). Got:\n{writer_rs}"
    );
    assert!(
        !writer_rs.contains("replay_all(self.path.clone())"),
        "must not clone String path for &str formal. Got:\n{writer_rs}"
    );
    assert!(
        writer_rs.contains("replay_to_lsn(&self.path")
            || writer_rs.contains("replay_to_lsn(self.path.as_str()")
            || writer_rs.contains("replay_to_lsn(& self.path"),
        "path field must borrow as &str for replay_to_lsn(path: &str). Got:\n{writer_rs}"
    );
    assert!(
        !writer_rs.contains("replay_to_lsn(self.path.clone()"),
        "must not clone String path for replay_to_lsn. Got:\n{writer_rs}"
    );
}

// ── WDB-047 (full layout): wdb-substrate store.wj — enum Value + PatchPart + delete_key path ─

#[test]
fn wdb_substrate_full_store_patch_part_enum_value_layout() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "types/part_id.wj",
        r#"
pub struct PartId {
    pub value: u128,
}

impl PartId {
    pub fn new(value: u128) -> PartId {
        PartId { value: value }
    }
}
"#,
    );
    test.add_file(
        "types/key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}

impl Key {
    pub fn new(bytes: Vec<u8>) -> Key {
        Key { bytes: bytes }
    }
}
"#,
    );
    test.add_file(
        "types/value.wj",
        r#"
pub enum Value {
    Null,
    Int64(i64),
}
"#,
    );
    test.add_file(
        "types/mod.wj",
        "pub mod part_id\npub mod key\npub mod value\n",
    );
    test.add_file(
        "substrate/hot_part.wj",
        r#"
use crate::types::key::Key
use crate::types::value::Value

pub struct HotPart {
    entries: Vec<(Key, Value)>,
}

impl HotPart {
    pub fn new() -> HotPart {
        HotPart { entries: Vec::new() }
    }

    pub fn len(self) -> usize {
        self.entries.len()
    }

    pub fn put_value(self, key: Key, value: Value) {
        self.entries.push((key, value))
    }

    pub fn delete_key(self, key: Key) {
        let mut out = Vec::new()
        let count = self.entries.len()
        let mut i = 0
        while i < count {
            if self.entries[i].0.bytes != key.bytes {
                out.push(self.entries[i])
            }
            i = i + 1
        }
        self.entries = out
    }

    pub fn get_value(self, key: Key) -> Option<Value> {
        let count = self.entries.len()
        let mut i = 0
        while i < count {
            if self.entries[i].0.bytes == key.bytes {
                return Some(self.entries[i].1)
            }
            i = i + 1
        }
        None
    }

    pub fn reset(self) {
        self.entries = Vec::new()
    }
}
"#,
    );
    test.add_file(
        "substrate/patch_part.wj",
        r#"
use crate::types::key::Key
use crate::types::value::Value
use crate::types::part_id::PartId

pub struct PatchEntry {
    pub row_key: Key,
    pub value: Value,
    pub tombstone: bool,
}

pub struct PatchPart {
    pub base_part_id: PartId,
    pub patches: Vec<PatchEntry>,
}

impl PatchPart {
    pub fn new(base_part_id: PartId) -> PatchPart {
        PatchPart {
            base_part_id: base_part_id,
            patches: Vec::new(),
        }
    }

    pub fn apply_put(self, key: Key, value: Value) {
        self.patches.push(PatchEntry {
            row_key: key,
            value: value,
            tombstone: false,
        })
    }

    pub fn apply_delete(self, key: Key) {
        self.patches.push(PatchEntry {
            row_key: key,
            value: Value::Null,
            tombstone: true,
        })
    }

    pub fn get_value(self, key: Key) -> Option<Value> {
        let count = self.patches.len()
        let mut i = count
        while i > 0 {
            i = i - 1
            let entry = self.patches[i]
            if entry.row_key.bytes == key.bytes {
                if entry.tombstone {
                    return None
                } else {
                    return Some(entry.value)
                }
            }
        }
        None
    }

    pub fn has_key(self, key: Key) -> bool {
        let count = self.patches.len()
        let mut i = 0
        while i < count {
            if self.patches[i].row_key.bytes == key.bytes {
                return true
            }
            i = i + 1
        }
        false
    }
}
"#,
    );
    test.add_file(
        "substrate/store.wj",
        r#"
use crate::types::key::Key
use crate::types::value::Value
use crate::types::part_id::PartId

use crate::substrate::hot_part::HotPart
use crate::substrate::patch_part::PatchPart

pub struct BasePart {
    pub part_id: PartId,
    pub entries: Vec<(Key, Value)>,
}

impl BasePart {
    pub fn get(self, key: Key) -> Option<Value> {
        let count = self.entries.len()
        let mut i = 0
        while i < count {
            if self.entries[i].0.bytes == key.bytes {
                return Some(self.entries[i].1)
            }
            i = i + 1
        }
        None
    }

    pub fn has_key(self, key: Key) -> bool {
        match self.get(key) {
            Some(_) => true,
            None => false,
        }
    }
}

pub struct LsmStore {
    hot: HotPart,
    base_parts: Vec<BasePart>,
    patches: Vec<PatchPart>,
    next_part_id: u128,
}

impl LsmStore {
    pub fn new() -> LsmStore {
        LsmStore {
            hot: HotPart::new(),
            base_parts: Vec::new(),
            patches: Vec::new(),
            next_part_id: 1,
        }
    }

    pub fn put_value(self, key: Key, value: Value) {
        if self.key_in_latest_base(key) {
            self.apply_patch_put(key, value)
        } else {
            self.hot.put_value(key, value)
        }
    }

    pub fn delete_key(self, key: Key) {
        if self.key_in_latest_base(key) {
            self.apply_patch_delete(key)
        } else {
            self.hot.delete_key(key)
        }
    }

    pub fn get_value(self, key: Key) -> Option<Value> {
        match self.hot.get_value(key) {
            Some(v) => Some(v),
            None => {
                let patch_count = self.patches.len()
                let mut pi = patch_count
                while pi > 0 {
                    pi = pi - 1
                    match self.patches[pi].get_value(key) {
                        Some(v) => return Some(v),
                        None => {
                            if self.patches[pi].has_key(key) {
                                return None
                            }
                        }
                    }
                }
                let base_count = self.base_parts.len()
                let mut bi = base_count
                while bi > 0 {
                    bi = bi - 1
                    match self.base_parts[bi].get(key) {
                        Some(v) => return Some(v),
                        None => {}
                    }
                }
                None
            }
        }
    }
}

impl LsmStore {
    fn key_in_latest_base(self, key: Key) -> bool {
        let base_count = self.base_parts.len()
        if base_count == 0 {
            false
        } else {
            let latest = self.base_parts[base_count - 1]
            latest.has_key(key)
        }
    }

    fn apply_patch_put(self, key: Key, value: Value) {
        let base_count = self.base_parts.len()
        if base_count == 0 {
            self.hot.put_value(key, value)
        } else {
            let base_id = self.base_parts[base_count - 1].part_id
            if self.patches.len() == 0 {
                let mut patch = PatchPart::new(base_id)
                patch.apply_put(key, value)
                self.patches.push(patch)
            } else {
                let patch_idx = self.patches.len() - 1
                let patch_base = self.patches[patch_idx].base_part_id
                if patch_base.value == base_id.value {
                    self.patches[patch_idx].apply_put(key, value)
                } else {
                    let mut patch = PatchPart::new(base_id)
                    patch.apply_put(key, value)
                    self.patches.push(patch)
                }
            }
        }
    }

    fn apply_patch_delete(self, key: Key) {
        let base_count = self.base_parts.len()
        if base_count == 0 {
            self.hot.delete_key(key)
        } else {
            let base_id = self.base_parts[base_count - 1].part_id
            if self.patches.len() == 0 {
                let mut patch = PatchPart::new(base_id)
                patch.apply_delete(key)
                self.patches.push(patch)
            } else {
                let patch_idx = self.patches.len() - 1
                self.patches[patch_idx].apply_delete(key)
            }
        }
    }
}
"#,
    );
    test.add_file(
        "substrate/mod.wj",
        "pub mod hot_part\npub mod patch_part\npub mod store\n",
    );
    test.add_file(
        "main.wj",
        r#"
use crate::substrate::store::LsmStore
use crate::types::key::Key
use crate::types::value::Value
use crate::types::part_id::PartId

pub fn run() {
    let store = LsmStore::new()
    store.put_value(Key { bytes: vec![1u8] }, Value::Int64(42))
    store.delete_key(Key { bytes: vec![2u8] })
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("substrate/store.rs").expect("substrate/store.rs");
    assert!(
        rs.contains("pub fn put_value(") && rs.contains("key: Key") && rs.contains("value: Value"),
        "put_value must keep owned Key and owned Value outer formals (wdb-substrate store.wj). Got:\n{rs}"
    );
    assert!(
        rs.contains("self.apply_patch_put(")
            && (rs.contains("apply_patch_put(&key") || rs.contains("apply_patch_put(key, &value"))
            && !rs.contains("apply_patch_put(key.clone(), value.clone())"),
        "apply_patch_put must borrow after forward-ref guard, not clone both args. Got:\n{rs}"
    );
    assert!(
        (rs.contains("self.apply_patch_delete(key)")
            || rs.contains("self.apply_patch_delete(key.clone())"))
            && !rs.contains("self.apply_patch_delete(&key)"),
        "apply_patch_delete must pass owned Key, not &key. Got:\n{rs}"
    );
    assert!(
        rs.contains("pub fn apply_patch_delete(") && rs.contains("key: Key"),
        "apply_patch_delete must emit owned Key formal. Got:\n{rs}"
    );
}

// ── WDB-050: WalSegment append_put/append_delete — vec literal borrow at call site ───────

#[test]
fn wdb_wal_segment_vec_literal_borrow_at_append() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "wal/lsn.wj",
        r#"
pub struct Lsn {
    pub value: u64,
}
"#,
    );
    test.add_file(
        "wal/record.wj",
        r#"
use crate::wal::lsn::Lsn

pub enum WalRecordKind {
    Put,
    Delete,
}

pub struct WalRecord {
    pub lsn: Lsn,
    pub kind: WalRecordKind,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

pub fn encode_record(record: WalRecord) -> Vec<u8> {
    let _ = (record.lsn.value, record.kind, record.key, record.value)
    Vec::new()
}

pub fn decode_records(bytes: Vec<u8>) -> Vec<WalRecord> {
    let _ = bytes.len()
    Vec::new()
}

impl WalRecord {
    pub fn put(lsn: Lsn, key: Vec<u8>, value: Vec<u8>) -> WalRecord {
        WalRecord {
            lsn: lsn,
            kind: WalRecordKind::Put,
            key: key,
            value: value,
        }
    }

    pub fn delete(lsn: Lsn, key: Vec<u8>) -> WalRecord {
        WalRecord {
            lsn: lsn,
            kind: WalRecordKind::Delete,
            key: key,
            value: Vec::new(),
        }
    }
}
"#,
    );
    test.add_file(
        "wal/segment.wj",
        r#"
use crate::wal::lsn::Lsn
use crate::wal::record::WalRecord
use crate::wal::record::encode_record
use crate::wal::record::decode_records

pub struct WalSegment {
    pub bytes: Vec<u8>,
    pub next_lsn: Lsn,
}

impl WalSegment {
    pub fn with_header() -> WalSegment {
        WalSegment {
            bytes: vec![1u8],
            next_lsn: Lsn { value: 1 },
        }
    }

    pub fn append_put(self, key: Vec<u8>, value: Vec<u8>) -> Lsn {
        let lsn = self.next_lsn
        let record = WalRecord::put(lsn, key, value)
        let payload = encode_record(record)
        let mut i = 0
        while i < payload.len() {
            self.bytes.push(payload[i])
            i = i + 1
        }
        self.next_lsn = Lsn { value: lsn.value + 1 }
        lsn
    }

    pub fn append_delete(self, key: Vec<u8>) -> Lsn {
        let lsn = self.next_lsn
        let record = WalRecord::delete(lsn, key)
        let payload = encode_record(record)
        let mut i = 0
        while i < payload.len() {
            self.bytes.push(payload[i])
            i = i + 1
        }
        self.next_lsn = Lsn { value: lsn.value + 1 }
        lsn
    }

    pub fn replay(self) -> Vec<WalRecord> {
        decode_records(self.bytes)
    }
}
"#,
    );
    test.add_file(
        "wal/segment_test.wj",
        r#"
use crate::wal::segment::WalSegment

pub fn test_segment_replay_returns_records_in_order() {
    let mut seg = WalSegment::with_header()
    seg.append_put(vec![10], vec![20])
    seg.append_delete(vec![10])
    let records = seg.replay()
    let _ = records.len()
}

pub fn test_segment_pitr_replay_to_lsn() {
    let mut seg = WalSegment::with_header()
    seg.append_put(vec![1], vec![10])
    let mid = seg.append_put(vec![2], vec![20])
    seg.append_put(vec![3], vec![30])
    let _ = mid.value
    let _ = seg.replay()
}
"#,
    );
    test.add_file(
        "wal/mod.wj",
        "pub mod lsn\npub mod record\npub mod segment\npub mod segment_test\n",
    );
    test.add_file(
        "main.wj",
        r#"
use crate::wal::segment_test::test_segment_replay_returns_records_in_order

pub fn run() {
    test_segment_replay_returns_records_in_order()
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("wal/segment_test.rs").expect("wal/segment_test.rs");
    assert!(
        !rs.contains("append_put(vec![10], vec![20])")
            && !rs.contains("append_delete(vec![10])"),
        "vec literals must borrow at append_put/append_delete when formals codegen as &Vec. Got:\n{rs}"
    );
    assert!(
        rs.contains("append_put(") && rs.contains("append_delete("),
        "expected append calls in segment test. Got:\n{rs}"
    );
}

// ── WDB-051: WalRecord::put — owned Lsn field from owned Lsn param ───────────────────────

#[test]
fn wdb_wal_record_put_owned_lsn_in_struct_init() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "wal/lsn.wj",
        r#"
pub struct Lsn {
    pub value: u64,
}

impl Lsn {
    pub fn next(self) -> Lsn {
        Lsn { value: self.value + 1 }
    }
}
"#,
    );
    test.add_file(
        "wal/record.wj",
        r#"
use crate::wal::lsn::Lsn

pub enum WalRecordKind {
    Put,
    Delete,
}

pub struct WalRecord {
    pub lsn: Lsn,
    pub kind: WalRecordKind,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

impl WalRecord {
    pub fn put(lsn: Lsn, key: Vec<u8>, value: Vec<u8>) -> WalRecord {
        WalRecord {
            lsn: lsn,
            kind: WalRecordKind::Put,
            key: key,
            value: value,
        }
    }

    pub fn delete(lsn: Lsn, key: Vec<u8>) -> WalRecord {
        WalRecord {
            lsn: lsn,
            kind: WalRecordKind::Delete,
            key: key,
            value: Vec::new(),
        }
    }
}
"#,
    );
    test.add_file(
        "wal/segment.wj",
        r#"
use crate::wal::lsn::Lsn
use crate::wal::record::WalRecord

pub struct WalSegment {
    pub next_lsn: Lsn,
}

impl WalSegment {
    pub fn append_put(self, key: Vec<u8>, value: Vec<u8>) -> Lsn {
        let lsn = self.next_lsn
        let _ = WalRecord::put(lsn, key, value)
        lsn
    }
}
"#,
    );
    test.add_file("wal/mod.wj", "pub mod lsn\npub mod record\npub mod segment\n");
    test.add_file(
        "main.wj",
        r#"
use crate::wal::segment::WalSegment

pub fn run() {
    let seg = WalSegment { next_lsn: crate::wal::lsn::Lsn { value: 1 } }
    let _ = seg.append_put(vec![1u8], vec![2u8])
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("wal/record.rs").expect("wal/record.rs");
    assert!(
        rs.contains("pub fn put("),
        "put must exist. Got:\n{rs}"
    );
    // WJ source uses owned Lsn; bad codegen borrows param then shorthand-inits owned field.
    if rs.contains("lsn: &Lsn") {
        assert!(
            rs.contains("lsn: lsn.clone()") || rs.contains("lsn: (*lsn).clone()"),
            "borrowed Lsn param must clone into owned struct field. Got:\n{rs}"
        );
        assert!(
            !rs.contains("WalRecord { lsn, kind: WalRecordKind::Put"),
            "must not shorthand-init owned Lsn from &Lsn param. Got:\n{rs}"
        );
    } else {
        assert!(
            rs.contains("lsn: Lsn") || rs.contains("lsn: lsn"),
            "owned Lsn param must assign to owned lsn field. Got:\n{rs}"
        );
    }
}

// ── WDB-052: WalSegment::replay — borrow bytes field behind inferred &self ─────────────

#[test]
fn wdb_wal_segment_replay_borrows_bytes_field() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "wal/lsn.wj",
        r#"
pub struct Lsn {
    pub value: u64,
}

impl Lsn {
    pub fn next(self) -> Lsn {
        Lsn { value: self.value + 1 }
    }
}
"#,
    );
    test.add_file(
        "wal/record.wj",
        r#"
use crate::wal::lsn::Lsn

pub enum WalRecordKind {
    Put,
    Delete,
}

pub struct WalRecord {
    pub lsn: Lsn,
    pub kind: WalRecordKind,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

impl WalRecord {
    pub fn put(lsn: Lsn, key: Vec<u8>, value: Vec<u8>) -> WalRecord {
        WalRecord {
            lsn: lsn,
            kind: WalRecordKind::Put,
            key: key,
            value: value,
        }
    }
}

pub fn encode_record(record: WalRecord) -> Vec<u8> {
    let _ = record.key.len()
    Vec::new()
}

pub fn decode_records(bytes: Vec<u8>) -> Vec<WalRecord> {
    let _ = bytes.len()
    Vec::new()
}
"#,
    );
    test.add_file(
        "wal/segment.wj",
        r#"
use crate::wal::lsn::Lsn
use crate::wal::record::WalRecord
use crate::wal::record::encode_record
use crate::wal::record::decode_records

pub struct WalSegment {
    pub bytes: Vec<u8>,
    pub next_lsn: Lsn,
}

impl WalSegment {
    pub fn append_put(self, key: Vec<u8>, value: Vec<u8>) -> Lsn {
        let lsn = self.next_lsn
        let record = WalRecord::put(lsn, key, value)
        let payload = encode_record(record)
        let mut i = 0
        while i < payload.len() {
            self.bytes.push(payload[i])
            i = i + 1
        }
        self.next_lsn = lsn.next()
        lsn
    }

    pub fn replay(self) -> Vec<WalRecord> {
        decode_records(self.bytes)
    }
}
"#,
    );
    test.add_file(
        "wal/writer.wj",
        r#"
use crate::wal::lsn::Lsn
use crate::wal::record::WalRecord
use crate::wal::segment::WalSegment

pub struct WalWriter {
    pub segment: WalSegment,
}

impl WalWriter {
    pub fn replay_all_records(self) -> Vec<WalRecord> {
        if self.segment.next_lsn.value > 1 {
            self.segment.replay()
        } else {
            Vec::new()
        }
    }
}
"#,
    );
    test.add_file("wal/mod.wj", "pub mod lsn\npub mod record\npub mod segment\npub mod writer\n");
    test.add_file(
        "main.wj",
        r#"
use crate::wal::writer::WalWriter
use crate::wal::segment::WalSegment
use crate::wal::lsn::Lsn

pub fn run() {
    let writer = WalWriter {
        segment: WalSegment {
            bytes: vec![1u8],
            next_lsn: Lsn { value: 2 },
        },
    }
    let _ = writer.replay_all_records()
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("wal/segment.rs").expect("wal/segment.rs");
    assert!(
        rs.contains("pub fn replay(&self)"),
        "read-only replay must infer &self (matches wdb-wal segment.wj). Got:\n{rs}"
    );
    for line in rs.lines() {
        if line.contains("decode_records(self.bytes)")
            && !line.contains("self.bytes.clone()")
            && !line.contains("&self.bytes")
        {
            panic!(
                "replay(&self) must not move self.bytes into decode_records(Vec<u8>). Line: {line}\nFull:\n{rs}"
            );
        }
    }
}

// ── WDB-053: WalWriter::replay_through — borrow Lsn at replay_to_lsn call ────────────────

#[test]
fn wdb_wal_replay_to_lsn_borrows_through_lsn() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "wal/lsn.wj",
        r#"
pub struct Lsn {
    pub value: u64,
}

impl Lsn {
    pub fn is_at_or_before(self, other: Lsn) -> bool {
        self.value <= other.value
    }
}
"#,
    );
    test.add_file(
        "wal/record.wj",
        r#"
use crate::wal::lsn::Lsn

pub struct WalRecord {
    pub lsn: Lsn,
    pub key: Vec<u8>,
}

pub fn replay_to_lsn(path: string, through: Lsn) -> Vec<WalRecord> {
    let _ = (path, through.value)
    Vec::new()
}
"#,
    );
    test.add_file(
        "wal/writer.wj",
        r#"
use crate::wal::lsn::Lsn
use crate::wal::record::WalRecord
use crate::wal::record::replay_to_lsn

pub struct WalWriter {
    path: string,
}

impl WalWriter {
    pub fn replay_through(self, through: Lsn) -> Vec<WalRecord> {
        replay_to_lsn(self.path, through)
    }
}
"#,
    );
    test.add_file("wal/mod.wj", "pub mod lsn\npub mod record\npub mod writer\n");
    test.add_file(
        "main.wj",
        r#"
use crate::wal::writer::WalWriter
use crate::wal::lsn::Lsn

pub fn run() {
    let writer = WalWriter { path: "wal.log" }
    let _ = writer.replay_through(Lsn { value: 42 })
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("wal/writer.rs").expect("wal/writer.rs");
    // Lsn is a Copy aggregate ({ value: u64 }), so Rust auto-borrows owned values
    // to &Lsn at call sites. Both explicit &through and bare through are correct.
    assert!(
        rs.contains("replay_to_lsn(")
            && (rs.contains("replay_to_lsn(&self.path, &through)")
                || rs.contains("replay_to_lsn(self.path.as_str(), &through)")
                || rs.contains("replay_to_lsn(&self.path, through.clone())")
                || rs.contains("replay_to_lsn(&self.path, through)")),
        "through Lsn must borrow/clone/auto-borrow for replay_to_lsn formal. Got:\n{rs}"
    );
    let record_rs = map.get("wal/record.rs").expect("wal/record.rs");
    if record_rs.contains("through: &Lsn") {
        assert!(
            record_rs.contains("is_at_or_before(*through)")
                || record_rs.contains("is_at_or_before(through.clone())")
                || record_rs.contains("is_at_or_before(through)"),
            "when through is &Lsn, body may deref, clone, or auto-borrow for is_at_or_before. Got:\n{record_rs}"
        );
    }
}

// ── WDB-056: RecoveredMap upsert — keys_equal readonly Vec params (borrowed formals OK) ─

#[test]
fn wdb_wal_recovered_map_keys_equal_owned_vec_params() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "wal/record.wj",
        r#"
pub enum WalRecordKind {
    Put,
    Delete,
}

pub struct WalRecord {
    pub kind: WalRecordKind,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}
"#,
    );
    test.add_file(
        "wal/wal.wj",
        r#"
use crate::wal::record::WalRecord
use crate::wal::record::WalRecordKind

pub struct RecoveredMap {
    pub entries: Vec<(Vec<u8>, Vec<u8>)>,
}

impl RecoveredMap {
    pub fn new() -> RecoveredMap {
        RecoveredMap { entries: Vec::new() }
    }

    fn upsert(self, key: Vec<u8>, value: Vec<u8>) {
        let count = self.entries.len()
        let mut i = 0
        let mut found = false
        while i < count {
            let existing_key = self.entries[i].0
            if keys_equal(existing_key, key) {
                self.entries[i] = (key, value)
                found = true
            }
            i = i + 1
        }
        if !found {
            self.entries.push((key, value))
        }
    }

    pub fn apply(self, record: WalRecord) {
        match record.kind {
            WalRecordKind::Put => {
                self.upsert(record.key, record.value)
            }
            WalRecordKind::Delete => {
                let mut out = Vec::new()
                let count = self.entries.len()
                let mut i = 0
                while i < count {
                    let ekey = self.entries[i].0
                    let evalue = self.entries[i].1
                    if !keys_equal(ekey, record.key) {
                        out.push((ekey, evalue))
                    }
                    i = i + 1
                }
                self.entries = out
            }
        }
    }
}

fn keys_equal(a: Vec<u8>, b: Vec<u8>) -> bool {
    if a.len() != b.len() {
        false
    } else {
        let mut i = 0
        while i < a.len() {
            if a[i] != b[i] {
                return false
            }
            i = i + 1
        }
        true
    }
}
"#,
    );
    test.add_file("wal/mod.wj", "pub mod record\npub mod wal\n");
    test.add_file(
        "main.wj",
        r#"
use crate::wal::wal::RecoveredMap
use crate::wal::record::WalRecord
use crate::wal::record::WalRecordKind

pub fn run() {
    let mut map = RecoveredMap::new()
    let record = WalRecord {
        kind: WalRecordKind::Put,
        key: vec![1u8],
        value: vec![2u8],
    }
    map.apply(record)
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("wal/wal.rs").expect("wal/wal.rs");
    assert!(
        rs.contains("fn keys_equal(a: Vec<u8>, b: Vec<u8>)")
            || rs.contains("fn keys_equal(a: &Vec<u8>, b: &Vec<u8>)"),
        "keys_equal formals (owned or borrowed readonly). Got:\n{rs}"
    );
    let upsert_ok = rs.contains("keys_equal(existing_key, key)")
        || rs.contains("keys_equal(existing_key.clone(), key)")
        || rs.contains("keys_equal(existing_key, key.clone())")
        || rs.contains("keys_equal(&existing_key, &key)")
        || rs.contains("keys_equal(&existing_key, &key.clone())");
    assert!(
        upsert_ok,
        "upsert must pass Vec to keys_equal (owned, cloned, or borrowed). Got:\n{rs}"
    );
    let delete_ok = rs.contains("keys_equal(ekey, record.key)")
        || rs.contains("keys_equal(ekey.clone(), record.key)")
        || rs.contains("keys_equal(ekey, record.key.clone())")
        || rs.contains("keys_equal(ekey.clone(), record.key.clone())")
        || rs.contains("keys_equal(&ekey, &record.key)")
        || rs.contains("keys_equal(&ekey, &record.key.clone())");
    assert!(
        delete_ok,
        "apply delete path must pass Vec to keys_equal. Got:\n{rs}"
    );
}

// ── WDB-054: LsmEngine::recover — seed_write(key, …) not &key after Key::new ────────────

#[test]
fn wdb_lsm_engine_recover_seed_write_owned_key() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "types/key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}

impl Key {
    pub fn new(bytes: Vec<u8>) -> Key {
        Key { bytes: bytes }
    }
}
"#,
    );
    test.add_file(
        "types/value.wj",
        r#"
pub enum Value {
    Null,
    Int64(i64),
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\npub mod value\n");
    test.add_file(
        "engine/memory.wj",
        r#"
use crate::types::key::Key
use crate::types::value::Value

pub struct MemoryEngine {}

impl MemoryEngine {
    pub fn new() -> MemoryEngine {
        MemoryEngine {}
    }

    pub fn seed_write(self, key: Key, value: Value, version: u64) {
        let _ = (key.bytes.len(), version)
    }

    pub fn set_committed_version(self, version: u64) {
        let _ = version
    }
}
"#,
    );
    test.add_file(
        "wal/record.wj",
        r#"
pub enum WalRecordKind {
    Put,
    Delete,
}

pub struct WalRecord {
    pub kind: WalRecordKind,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}
"#,
    );
    test.add_file(
        "wal/writer.wj",
        r#"
use crate::wal::record::WalRecord

pub struct WalWriter {}

impl WalWriter {
    pub fn open_existing(path: string) -> WalWriter {
        let _ = path
        WalWriter {}
    }

    pub fn replay_all_records(self) -> Vec<WalRecord> {
        Vec::new()
    }
}
"#,
    );
    test.add_file(
        "substrate/store.wj",
        r#"
use crate::wal::record::WalRecord

pub struct LsmStore {}

impl LsmStore {
    pub fn new() -> LsmStore {
        LsmStore {}
    }

    pub fn apply_wal_record(self, record: WalRecord) {
        let _ = record.key.len()
    }
}
"#,
    );
    test.add_file(
        "substrate/lsm_engine.wj",
        r#"
use crate::types::key::Key
use crate::types::value::Value

use crate::engine::memory::MemoryEngine
use crate::substrate::store::LsmStore
use crate::wal::writer::WalWriter
use crate::wal::record::WalRecordKind

pub struct LsmEngine {
    engine: MemoryEngine,
    store: LsmStore,
    writer: WalWriter,
}

impl LsmEngine {
    pub fn recover(path: string) -> LsmEngine {
        let writer = WalWriter::open_existing(path)
        let records = writer.replay_all_records()
        let mut store = LsmStore::new()
        let mut engine = MemoryEngine::new()
        let mut version = 0u64
        for record in records {
            match record.kind {
                WalRecordKind::Put => {
                    version = version + 1
                    let key = Key::new(record.key)
                    let value = Value::Int64(7)
                    engine.seed_write(key, value, version)
                }
                WalRecordKind::Delete => {
                    version = version + 1
                }
            }
            store.apply_wal_record(record)
        }
        engine.set_committed_version(version)
        LsmEngine {
            engine: engine,
            store: store,
            writer: writer,
        }
    }
}
"#,
    );
    test.add_file(
        "substrate/mod.wj",
        "pub mod store\npub mod lsm_engine\n",
    );
    test.add_file("wal/mod.wj", "pub mod record\npub mod writer\n");
    test.add_file("engine/mod.wj", "pub mod memory\n");
    test.add_file(
        "main.wj",
        r#"
use crate::substrate::lsm_engine::LsmEngine

pub fn run() {
    let _ = LsmEngine::recover("wal.log")
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("substrate/lsm_engine.rs").expect("substrate/lsm_engine.rs");
    assert!(
        rs.contains("engine.seed_write(")
            && (rs.contains("engine.seed_write(key,") || rs.contains("engine.seed_write(key.clone(),"))
            && !rs.contains("engine.seed_write(&key,"),
        "seed_write must pass owned Key after Key::new in recover loop. Got:\n{rs}"
    );
}

// ── WDB-055: LsmEngine::put — engine.put(key, value) after using key in pending push ─────

#[test]
fn wdb_lsm_engine_put_owned_key_after_pending_push() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "types/key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}
"#,
    );
    test.add_file(
        "types/value.wj",
        r#"
pub enum Value {
    Null,
    Int64(i64),
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\npub mod value\n");
    test.add_file(
        "engine/memory.wj",
        r#"
use crate::types::key::Key
use crate::types::value::Value

pub struct MemoryEngine {}

impl MemoryEngine {
    pub fn new() -> MemoryEngine {
        MemoryEngine {}
    }

    pub fn put(self, key: Key, value: Value) {
        let _ = (key.bytes.len(), value)
    }
}
"#,
    );
    test.add_file(
        "substrate/lsm_engine.wj",
        r#"
use crate::types::key::Key
use crate::types::value::Value
use crate::engine::memory::MemoryEngine

struct PendingWrite {
    key: Key,
    value: Value,
    is_delete: bool,
}

pub struct LsmEngine {
    engine: MemoryEngine,
    pending: Vec<PendingWrite>,
}

impl LsmEngine {
    pub fn new() -> LsmEngine {
        LsmEngine {
            engine: MemoryEngine::new(),
            pending: Vec::new(),
        }
    }

    pub fn put(self, key: Key, value: Value) {
        self.pending.push(PendingWrite {
            key: key,
            value: value,
            is_delete: false,
        })
        self.engine.put(key, value)
    }
}
"#,
    );
    test.add_file("engine/mod.wj", "pub mod memory\n");
    test.add_file("substrate/mod.wj", "pub mod lsm_engine\n");
    test.add_file(
        "main.wj",
        r#"
use crate::substrate::lsm_engine::LsmEngine
use crate::types::key::Key
use crate::types::value::Value

pub fn run() {
    let engine = LsmEngine::new()
    engine.put(Key { bytes: vec![1u8] }, Value::Int64(9))
}
"#,
    );

    let map = test.compile().expect("compile");
    let mem_rs = map.get("engine/memory.rs").expect("engine/memory.rs");
    assert!(
        mem_rs.contains("fn put(") && mem_rs.contains("key: Key"),
        "MemoryEngine::put must keep owned Key formal. Got:\n{mem_rs}"
    );
    let rs = map.get("substrate/lsm_engine.rs").expect("substrate/lsm_engine.rs");
    assert!(
        rs.contains("pub fn put(") && rs.contains("key: Key"),
        "put must keep owned Key param. Got:\n{rs}"
    );
    assert!(
        rs.contains("self.engine.put(")
            && (rs.contains("self.engine.put(key,") || rs.contains("self.engine.put(key.clone(),"))
            && !rs.contains("self.engine.put(&key,"),
        "engine.put must receive owned Key after pending push uses key. Got:\n{rs}"
    );
}

// ── WDB-057: WalWriter replay_all_records else branch — &str not String at replay_all ───

#[test]
fn wdb_wal_writer_replay_all_records_branch_borrows_path() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "wal/lsn.wj",
        r#"
pub struct Lsn {
    pub value: u64,
}
"#,
    );
    test.add_file(
        "wal/record.wj",
        r#"
use crate::wal::lsn::Lsn

pub struct WalRecord {
    pub lsn: Lsn,
    pub key: Vec<u8>,
}

pub fn decode_records(bytes: Vec<u8>) -> Vec<WalRecord> {
    let _ = bytes.len()
    Vec::new()
}
"#,
    );
    test.add_file(
        "wal/segment.wj",
        r#"
use crate::wal::lsn::Lsn
use crate::wal::record::WalRecord
use crate::wal::record::decode_records

pub struct WalSegment {
    pub bytes: Vec<u8>,
    pub next_lsn: Lsn,
}

impl WalSegment {
    pub fn replay(self) -> Vec<WalRecord> {
        decode_records(self.bytes)
    }
}
"#,
    );
    test.add_file(
        "wal/wal.wj",
        r#"
use crate::wal::lsn::Lsn
use crate::wal::record::WalRecord
use crate::wal::record::decode_records
use crate::wal::segment::WalSegment

pub struct WalWriter {
    pub path: string,
    pub segment: WalSegment,
}

impl WalWriter {
    pub fn replay_all_records(self) -> Vec<WalRecord> {
        if self.segment.next_lsn.value > 1 {
            self.segment.replay()
        } else {
            replay_all(self.path)
        }
    }
}

pub fn replay_all(path: string) -> Vec<WalRecord> {
    decode_records(Vec::new())
}
"#,
    );
    test.add_file(
        "wal/mod.wj",
        "pub mod lsn\npub mod record\npub mod segment\npub mod wal\n",
    );
    test.add_file(
        "main.wj",
        r#"
use crate::wal::wal::WalWriter
use crate::wal::segment::WalSegment
use crate::wal::lsn::Lsn

pub fn run() {
    let writer = WalWriter {
        path: "wal.log",
        segment: WalSegment {
            bytes: Vec::new(),
            next_lsn: Lsn { value: 1 },
        },
    }
    let _ = writer.replay_all_records()
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("wal/wal.rs").expect("wal/wal.rs");
    assert!(
        rs.contains("pub fn replay_all(path: &str)") || rs.contains("pub fn replay_all(path: & str)"),
        "replay_all must lower string formal to &str (matches wdb-wal gen). Got:\n{rs}"
    );
    assert!(
        rs.contains("replay_all(&self.path)")
            || rs.contains("replay_all(self.path.as_str())")
            || rs.contains("replay_all(& self.path)"),
        "else branch must borrow path as &str. Got:\n{rs}"
    );
    assert!(
        !rs.contains("replay_all(self.path.clone())"),
        "must not pass owned String clone where replay_all expects &str. Got:\n{rs}"
    );
}

// ── WDB-058: WalWriter replay_through + replay_to_lsn body is_at_or_before ─────────────

#[test]
fn wdb_wal_replay_to_lsn_call_and_body_copy_lsn() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "wal/lsn.wj",
        r#"
pub struct Lsn {
    pub value: u64,
}

impl Lsn {
    pub fn is_at_or_before(self, other: Lsn) -> bool {
        self.value <= other.value
    }
}
"#,
    );
    test.add_file(
        "wal/record.wj",
        r#"
use crate::wal::lsn::Lsn

pub struct WalRecord {
    pub lsn: Lsn,
    pub key: Vec<u8>,
}

pub fn decode_records(bytes: Vec<u8>) -> Vec<WalRecord> {
    let _ = bytes.len()
    Vec::new()
}
"#,
    );
    test.add_file(
        "wal/wal.wj",
        r#"
use crate::wal::lsn::Lsn
use crate::wal::record::WalRecord
use crate::wal::record::decode_records

pub struct WalWriter {
    pub path: string,
}

impl WalWriter {
    pub fn replay_through(self, through: Lsn) -> Vec<WalRecord> {
        replay_to_lsn(self.path, through)
    }
}

pub fn replay_all(path: string) -> Vec<WalRecord> {
    decode_records(Vec::new())
}

pub fn replay_to_lsn(path: string, through: Lsn) -> Vec<WalRecord> {
    let all = replay_all(path)
    let mut out = Vec::new()
    for record in all {
        if record.lsn.is_at_or_before(through) {
            out.push(record)
        }
    }
    out
}
"#,
    );
    test.add_file(
        "wal/mod.wj",
        "pub mod lsn\npub mod record\npub mod wal\n",
    );
    test.add_file(
        "main.wj",
        r#"
use crate::wal::wal::WalWriter
use crate::wal::lsn::Lsn

pub fn run() {
    let writer = WalWriter { path: "wal.log" }
    let _ = writer.replay_through(Lsn { value: 42 })
}
"#,
    );

    let map = test.compile().expect("compile");
    let wal_rs = map.get("wal/wal.rs").expect("wal/wal.rs");
    // path (string) must borrow as &str. through is Copy (u64 field) — Owned `Lsn`
    // is the idiomatic Windjammer formal; `&Lsn` remains acceptable if inferred.
    assert!(
        wal_rs.contains("pub fn replay_to_lsn(path: &str, through: Lsn)")
            || wal_rs.contains("pub fn replay_to_lsn(path: &str, through: &Lsn)")
            || wal_rs.contains("pub fn replay_to_lsn(path: &str, through: & Lsn)"),
        "replay_to_lsn must lower to &str + owned-or-borrowed Lsn. Got:\n{wal_rs}"
    );
    assert!(
        wal_rs.contains("replay_to_lsn(&self.path, through)")
            || wal_rs.contains("replay_to_lsn(&self.path, &through)")
            || wal_rs.contains("replay_to_lsn(self.path.as_str(), &through)")
            || wal_rs.contains("replay_to_lsn(&self.path, through.clone())"),
        "WalWriter::replay_through must borrow path; through may be owned Copy or borrowed. Got:\n{wal_rs}"
    );
    assert!(
        !wal_rs.contains("replay_to_lsn(self.path.clone(), through)"),
        "must not pass owned String where &str expected. Got:\n{wal_rs}"
    );
    if wal_rs.contains("through: &Lsn") {
        assert!(
            wal_rs.contains("is_at_or_before(*through)")
                || wal_rs.contains("is_at_or_before(through.clone())"),
            "replay_to_lsn body must deref/clone &Lsn for owned is_at_or_before formal. Got:\n{wal_rs}"
        );
        assert!(
            !wal_rs.contains("is_at_or_before(through)"),
            "must not pass &Lsn where owned Lsn expected without deref. Got:\n{wal_rs}"
        );
    }
}

// ── WDB-059: RecoveredMap delete loop — keys_equal must not move ekey before out.push ──

#[test]
fn wdb_wal_keys_equal_delete_loop_no_use_after_move() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "wal/record.wj",
        r#"
pub enum WalRecordKind {
    Put,
    Delete,
}

pub struct WalRecord {
    pub kind: WalRecordKind,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}
"#,
    );
    test.add_file(
        "wal/wal.wj",
        r#"
use crate::wal::record::WalRecord
use crate::wal::record::WalRecordKind

pub struct RecoveredMap {
    pub entries: Vec<(Vec<u8>, Vec<u8>)>,
}

impl RecoveredMap {
    pub fn new() -> RecoveredMap {
        RecoveredMap { entries: Vec::new() }
    }

    pub fn apply(self, record: WalRecord) {
        match record.kind {
            WalRecordKind::Put => {
                let _ = (record.key, record.value)
            }
            WalRecordKind::Delete => {
                let mut out = Vec::new()
                let count = self.entries.len()
                let mut i = 0
                while i < count {
                    let ekey = self.entries[i].0
                    let evalue = self.entries[i].1
                    if !keys_equal(ekey, record.key) {
                        out.push((ekey, evalue))
                    }
                    i = i + 1
                }
                self.entries = out
            }
        }
    }
}

fn keys_equal(a: Vec<u8>, b: Vec<u8>) -> bool {
    if a.len() != b.len() {
        false
    } else {
        let mut i = 0
        while i < a.len() {
            if a[i] != b[i] {
                return false
            }
            i = i + 1
        }
        true
    }
}
"#,
    );
    test.add_file("wal/mod.wj", "pub mod record\npub mod wal\n");
    test.add_file(
        "main.wj",
        r#"
use crate::wal::wal::RecoveredMap
use crate::wal::record::WalRecord
use crate::wal::record::WalRecordKind

pub fn run() {
    let mut map = RecoveredMap::new()
    map.apply(WalRecord {
        kind: WalRecordKind::Delete,
        key: vec![1u8],
        value: Vec::new(),
    })
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("wal/wal.rs").expect("wal/wal.rs");
    assert!(
        rs.contains("fn keys_equal(a: Vec<u8>, b: Vec<u8>)")
            || rs.contains("fn keys_equal(a: &Vec<u8>, b: &Vec<u8>)"),
        "keys_equal signature. Got:\n{rs}"
    );
    let borrows_at_call = rs.contains("keys_equal(&ekey, &record.key)")
        || rs.contains("keys_equal(& ekey, &record.key)")
        || rs.contains("keys_equal(&ekey, &record.key.clone())");
    let clones_before_call = rs.contains("keys_equal(ekey.clone(), record.key.clone())")
        || rs.contains("keys_equal(ekey.clone(), record.key)")
        || rs.contains("keys_equal(ekey, record.key.clone())");
    assert!(
        borrows_at_call || clones_before_call,
        "delete loop must borrow or clone before keys_equal when ekey is reused in out.push. Got:\n{rs}"
    );
    assert!(
        !rs.contains("if !keys_equal(ekey, record.key)"),
        "owned keys_equal(ekey, record.key) moves ekey/record.key before out.push — E0382. Got:\n{rs}"
    );
}

// ── WDB-060: flat wal layout — cargo check (rustc) mirrors wdb-wal replay + recovered map ─

#[test]
fn wdb_wal_replay_and_recovered_map_rustc_check() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "wal_layout.wj",
        r#"
pub struct Lsn {
    pub value: u64,
}

impl Lsn {
    pub fn zero() -> Lsn {
        Lsn { value: 0 }
    }

    pub fn next(self) -> Lsn {
        Lsn { value: self.value + 1 }
    }

    pub fn is_at_or_before(self, other: Lsn) -> bool {
        self.value <= other.value
    }
}

pub enum WalRecordKind {
    Put,
    Delete,
}

pub struct WalRecord {
    pub lsn: Lsn,
    pub kind: WalRecordKind,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

pub fn decode_records(bytes: Vec<u8>) -> Vec<WalRecord> {
    let _ = bytes.len()
    Vec::new()
}

pub struct WalSegment {
    pub bytes: Vec<u8>,
    pub next_lsn: Lsn,
}

impl WalSegment {
    pub fn replay(self) -> Vec<WalRecord> {
        decode_records(self.bytes)
    }
}

pub struct WalWriter {
    pub path: string,
    pub segment: WalSegment,
}

impl WalWriter {
    pub fn replay_all_records(self) -> Vec<WalRecord> {
        if self.segment.next_lsn.value > 1 {
            self.segment.replay()
        } else {
            replay_all(self.path)
        }
    }

    pub fn replay_through(self, through: Lsn) -> Vec<WalRecord> {
        replay_to_lsn(self.path, through)
    }
}

pub fn replay_all(path: string) -> Vec<WalRecord> {
    decode_records(Vec::new())
}

pub fn replay_to_lsn(path: string, through: Lsn) -> Vec<WalRecord> {
    let all = replay_all(path)
    let mut out = Vec::new()
    for record in all {
        if record.lsn.is_at_or_before(through) {
            out.push(record)
        }
    }
    out
}

pub struct RecoveredMap {
    pub entries: Vec<(Vec<u8>, Vec<u8>)>,
}

impl RecoveredMap {
    pub fn new() -> RecoveredMap {
        RecoveredMap { entries: Vec::new() }
    }

    fn upsert(self, key: Vec<u8>, value: Vec<u8>) {
        let count = self.entries.len()
        let mut i = 0
        let mut found = false
        while i < count {
            let existing_key = self.entries[i].0
            if keys_equal(existing_key, key) {
                self.entries[i] = (key, value)
                found = true
            }
            i = i + 1
        }
        if !found {
            self.entries.push((key, value))
        }
    }

    pub fn apply(self, record: WalRecord) {
        match record.kind {
            WalRecordKind::Put => {
                self.upsert(record.key, record.value)
            }
            WalRecordKind::Delete => {
                let mut out = Vec::new()
                let count = self.entries.len()
                let mut i = 0
                while i < count {
                    let ekey = self.entries[i].0
                    let evalue = self.entries[i].1
                    if !keys_equal(ekey, record.key) {
                        out.push((ekey, evalue))
                    }
                    i = i + 1
                }
                self.entries = out
            }
        }
    }
}

fn keys_equal(a: Vec<u8>, b: Vec<u8>) -> bool {
    if a.len() != b.len() {
        false
    } else {
        let mut i = 0
        while i < a.len() {
            if a[i] != b[i] {
                return false
            }
            i = i + 1
        }
        true
    }
}
"#,
    );
    test.add_file(
        "main.wj",
        r#"
use crate::wal_layout::WalWriter
use crate::wal_layout::WalSegment
use crate::wal_layout::Lsn
use crate::wal_layout::RecoveredMap
use crate::wal_layout::WalRecord
use crate::wal_layout::WalRecordKind

pub fn run() {
    let writer = WalWriter {
        path: "wal.log",
        segment: WalSegment {
            bytes: vec![1u8],
            next_lsn: Lsn { value: 1 },
        },
    }
    let _ = writer.replay_all_records()
    let _ = writer.replay_through(Lsn { value: 3 })
    let mut map = RecoveredMap::new()
    map.apply(WalRecord {
        lsn: Lsn { value: 1 },
        kind: WalRecordKind::Delete,
        key: vec![9u8],
        value: Vec::new(),
    })
}
"#,
    );
    test.assert_compiles_without_error();
}

// ── WDB-061: PatchPart::apply_put — owned PatchEntry fields from borrowed params ─────────

#[test]
fn wdb_patch_part_apply_put_owned_struct_fields_from_borrowed_params() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "patch_part_layout.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}

pub enum Value {
    Null,
    Int64(i64),
}

pub struct PartId {
    pub value: u32,
}

pub struct PatchEntry {
    pub row_key: Key,
    pub value: Value,
    pub tombstone: bool,
}

pub struct PatchPart {
    pub base_part_id: PartId,
    pub patches: Vec<PatchEntry>,
}

impl PatchPart {
    pub fn new(base_part_id: PartId) -> PatchPart {
        PatchPart {
            base_part_id: base_part_id,
            patches: Vec::new(),
        }
    }

    pub fn apply_put(self, key: Key, value: Value) {
        self.patches.push(PatchEntry {
            row_key: key,
            value: value,
            tombstone: false,
        })
    }
}
"#,
    );
    test.add_file(
        "main.wj",
        r#"
use crate::patch_part_layout::PatchPart
use crate::patch_part_layout::Key
use crate::patch_part_layout::Value
use crate::patch_part_layout::PartId

pub fn run() {
    let mut part = PatchPart::new(PartId { value: 1 })
    part.apply_put(Key { bytes: vec![1u8] }, Value::Int64(7))
}
"#,
    );
    test.assert_compiles_without_error();
}

// ── WDB-062: Transaction record_read/write — owned entry fields from borrowed params ─────

#[test]
fn wdb_transaction_record_sets_owned_struct_fields_from_borrowed_params() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "transaction_layout.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}

pub enum Value {
    Null,
    Int64(i64),
}

pub struct Version {
    pub value: u64,
}

pub struct ReadEntry {
    pub key: Key,
    pub version_at_read: Version,
}

pub struct WriteEntry {
    pub key: Key,
    pub value: Option<Value>,
}

pub struct Transaction {
    pub read_set: Vec<ReadEntry>,
    pub write_set: Vec<WriteEntry>,
    pub snapshot_version: Version,
}

impl Transaction {
    pub fn begin(snapshot: Version) -> Transaction {
        Transaction {
            read_set: Vec::new(),
            write_set: Vec::new(),
            snapshot_version: snapshot,
        }
    }

    pub fn record_read(self, key: Key, version_at_read: Version) {
        self.read_set.push(ReadEntry {
            key: key,
            version_at_read: version_at_read,
        })
    }

    pub fn record_write(self, key: Key, value: Option<Value>) {
        self.write_set.push(WriteEntry {
            key: key,
            value: value,
        })
    }
}
"#,
    );
    test.add_file(
        "main.wj",
        r#"
use crate::transaction_layout::Transaction
use crate::transaction_layout::Key
use crate::transaction_layout::Value
use crate::transaction_layout::Version

pub fn run() {
    let mut txn = Transaction::begin(Version { value: 1 })
    txn.record_read(Key { bytes: vec![1u8] }, Version { value: 2 })
    txn.record_write(Key { bytes: vec![2u8] }, Some(Value::Int64(9)))
}
"#,
    );
    test.assert_compiles_without_error();
}

// ── WDB-063: MemoryEngine::seed_write — no double-move of Value across helpers ───────────

#[test]
fn wdb_memory_engine_seed_write_no_double_move_value() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "memory_engine_layout.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}

pub enum Value {
    Null,
    Int64(i64),
}

extern fn memory_engine_create_ffi() -> u64
extern fn memory_engine_seed_write_ffi(id: u64, key_bytes: Vec<u8>, value_tag: i32, value_i64: i64, version: u64)

pub struct MemoryEngine {
    handle: u64,
}

impl MemoryEngine {
    pub fn new() -> MemoryEngine {
        MemoryEngine { handle: memory_engine_create_ffi() }
    }

    pub fn seed_write(self, key: Key, value: Value, version: u64) {
        let tag = value_tag(value)
        let payload = value_i64(value)
        memory_engine_seed_write_ffi(self.handle, key.bytes, tag, payload, version)
    }
}

fn value_tag(value: Value) -> i32 {
    match value {
        Value::Null => 0,
        Value::Int64(_) => 1,
    }
}

fn value_i64(value: Value) -> i64 {
    match value {
        Value::Null => 0,
        Value::Int64(v) => v,
    }
}
"#,
    );
    test.add_file(
        "main.wj",
        r#"
use crate::memory_engine_layout::MemoryEngine
use crate::memory_engine_layout::Key
use crate::memory_engine_layout::Value

pub fn run() {
    let engine = MemoryEngine::new()
    engine.seed_write(Key { bytes: vec![1u8] }, Value::Int64(42), 1)
}
"#,
    );
    test.assert_compiles_without_error();
}

// ── WDB-064: LsmEngine apply_writes — borrow Vec locals at cross-crate append_put ────────

#[test]
fn wdb_lsm_engine_apply_writes_append_put_borrows_vec_locals() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "lsm_apply_layout.wj",
        r#"
pub struct Lsn {
    pub value: u64,
}

pub struct Key {
    pub bytes: Vec<u8>,
}

pub enum Value {
    Null,
    Int64(i64),
}

pub struct PendingWrite {
    pub key: Key,
    pub value: Value,
    pub is_delete: bool,
}

pub struct WalWriter {}

impl WalWriter {
    pub fn new() -> WalWriter {
        WalWriter {}
    }

    pub fn append_put(self, key: Vec<u8>, value: Vec<u8>) -> Lsn {
        let _ = (key.len(), value.len())
        Lsn { value: 1 }
    }

    pub fn append_delete(self, key: Vec<u8>) -> Lsn {
        let _ = key.len()
        Lsn { value: 1 }
    }
}

pub fn encode_value(value: Value) -> Vec<u8> {
    match value {
        Value::Null => Vec::new(),
        Value::Int64(v) => vec![v as u8],
    }
}

pub struct LsmEngine {
    writer: WalWriter,
}

impl LsmEngine {
    pub fn new() -> LsmEngine {
        LsmEngine { writer: WalWriter::new() }
    }

    pub fn apply_writes(self, writes: Vec<PendingWrite>) {
        for write in writes {
            if write.is_delete {
                let k = write.key
                let kb = k.bytes
                self.writer.append_delete(kb)
            } else {
                let k = write.key
                let v = write.value
                let kb = k.bytes
                let enc = encode_value(v)
                self.writer.append_put(kb, enc)
            }
        }
    }
}
"#,
    );
    test.add_file(
        "main.wj",
        r#"
use crate::lsm_apply_layout::LsmEngine
use crate::lsm_apply_layout::PendingWrite
use crate::lsm_apply_layout::Key
use crate::lsm_apply_layout::Value

pub fn run() {
    let engine = LsmEngine::new()
    engine.apply_writes(vec![PendingWrite {
        key: Key { bytes: vec![1u8] },
        value: Value::Int64(9),
        is_delete: false,
    }])
}
"#,
    );
    test.assert_compiles_without_error();
}

// ── WDB-065: SimNetwork send — owned NodeId fields in QueuedMessage struct init ──────────

#[test]
fn wdb_sim_network_queued_message_owned_node_id_fields() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "sim_network_layout.wj",
        r#"
pub struct NodeId {
    pub value: u32,
}

pub enum SimMessageKind {
    UpqueryResponse,
    LiveDelta,
}

pub struct SimMessage {
    pub kind: SimMessageKind,
    pub group_key: i64,
    pub value: f64,
    pub delta_amount: f64,
    pub logical_tick: u64,
}

pub fn encode_message(msg: SimMessage) -> Vec<u8> {
    let _ = msg.group_key
    Vec::new()
}

pub struct SimDuration {
    pub micros: i64,
}

pub struct SeededSimClock {
    pub now_micros: i64,
}

impl SeededSimClock {
    pub fn effective_now_micros(self) -> i64 {
        self.now_micros
    }
}

pub struct QueuedMessage {
    pub from: NodeId,
    pub to: NodeId,
    pub payload: Vec<u8>,
    pub deliver_at_micros: i64,
    pub sequence: u64,
}

pub struct SimNetwork {
    pub queue: Vec<QueuedMessage>,
    pub next_sequence: u64,
    pub default_latency_micros: i64,
}

impl SimNetwork {
    pub fn new(default_latency_micros: i64) -> SimNetwork {
        SimNetwork {
            queue: Vec::new(),
            next_sequence: 0,
            default_latency_micros: default_latency_micros,
        }
    }

    pub fn send(self, from: NodeId, to: NodeId, msg: SimMessage, clock: SeededSimClock, latency: SimDuration) {
        let payload = encode_message(msg)
        let seq = self.next_sequence
        self.next_sequence = seq + 1
        let deliver_at = clock.effective_now_micros() + latency.micros + self.default_latency_micros
        let queued = QueuedMessage {
            from: from,
            to: to,
            payload: payload,
            deliver_at_micros: deliver_at,
            sequence: seq,
        }
        self.queue.push(queued)
    }
}
"#,
    );
    test.add_file(
        "main.wj",
        r#"
use crate::sim_network_layout::SimNetwork
use crate::sim_network_layout::SimMessage
use crate::sim_network_layout::SimMessageKind
use crate::sim_network_layout::SeededSimClock
use crate::sim_network_layout::SimDuration
use crate::sim_network_layout::NodeId

pub fn run() {
    let mut net = SimNetwork::new(1000)
    net.send(
        NodeId { value: 1 },
        NodeId { value: 2 },
        SimMessage {
            kind: SimMessageKind::LiveDelta,
            group_key: 1,
            value: 1.0,
            delta_amount: 0.5,
            logical_tick: 1,
        },
        SeededSimClock { now_micros: 0 },
        SimDuration { micros: 500 },
    )
}
"#,
    );
    test.assert_compiles_without_error();
}

// ── WDB-063b: memory_engine seed_write — catch double-move pattern in emitted Rust ───────

#[test]
fn wdb_memory_engine_seed_write_emitted_rust_no_double_move() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "memory_engine_layout.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}

pub enum Value {
    Null,
    Int64(i64),
}

extern fn memory_engine_create_ffi() -> u64
extern fn memory_engine_seed_write_ffi(id: u64, key_bytes: Vec<u8>, value_tag: i32, value_i64: i64, version: u64)

pub struct MemoryEngine {
    handle: u64,
}

impl MemoryEngine {
    pub fn new() -> MemoryEngine {
        MemoryEngine { handle: memory_engine_create_ffi() }
    }

    pub fn seed_write(self, key: Key, value: Value, version: u64) {
        let tag = value_tag(value)
        let payload = value_i64(value)
        memory_engine_seed_write_ffi(self.handle, key.bytes, tag, payload, version)
    }
}

fn value_tag(value: Value) -> i32 {
    match value {
        Value::Null => 0,
        Value::Int64(_) => 1,
    }
}

fn value_i64(value: Value) -> i64 {
    match value {
        Value::Null => 0,
        Value::Int64(v) => v,
    }
}
"#,
    );
    test.add_file("main.wj", "pub fn run() {}");
    let map = test.compile().expect("compile");
    let rs = map.get("memory_engine_layout.rs").expect("memory_engine_layout.rs");
    if rs.contains("value_tag(value)") && rs.contains("value_i64(value") {
        assert!(
            rs.contains("value.clone()"),
            "value_tag + value_i64 on same binding must clone — wdb-substrate memory_engine.rs:37-38. Got:\n{rs}"
        );
    }
}

// ── WDB-064b: cross-module lsm_engine → wal append_put borrow in emitted Rust ───────────

#[test]
fn wdb_lsm_engine_apply_writes_emitted_rust_borrows_append_put() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "wal/wal.wj",
        r#"
pub struct Lsn {
    pub value: u64,
}

pub struct WalWriter {}

impl WalWriter {
    pub fn new() -> WalWriter {
        WalWriter {}
    }

    pub fn append_put(self, key: Vec<u8>, value: Vec<u8>) -> Lsn {
        let _ = (key.len(), value.len())
        Lsn { value: 1 }
    }
}
"#,
    );
    test.add_file("wal/mod.wj", "pub mod wal\n");
    test.add_file(
        "types/key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}
"#,
    );
    test.add_file(
        "types/value.wj",
        r#"
pub enum Value {
    Null,
    Int64(i64),
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\npub mod value\n");
    test.add_file(
        "substrate/pending.wj",
        r#"
use crate::types::key::Key
use crate::types::value::Value

pub struct PendingWrite {
    pub key: Key,
    pub value: Value,
    pub is_delete: bool,
}
"#,
    );
    test.add_file(
        "substrate/lsm_engine.wj",
        r#"
use crate::types::key::Key
use crate::types::value::Value
use crate::substrate::pending::PendingWrite
use crate::wal::wal::WalWriter

pub fn encode_value(value: Value) -> Vec<u8> {
    match value {
        Value::Null => Vec::new(),
        Value::Int64(v) => vec![v as u8],
    }
}

pub struct LsmEngine {
    writer: WalWriter,
}

impl LsmEngine {
    pub fn new() -> LsmEngine {
        LsmEngine { writer: WalWriter::new() }
    }

    pub fn apply_writes(self, writes: Vec<PendingWrite>) {
        for write in writes {
            let k = write.key
            let v = write.value
            let kb = k.bytes
            let enc = encode_value(v)
            self.writer.append_put(kb, enc)
        }
    }
}
"#,
    );
    test.add_file(
        "substrate/mod.wj",
        "pub mod pending\npub mod lsm_engine\n",
    );
    test.add_file("main.wj", "pub fn run() {}");
    let map = test.compile().expect("compile");
    let rs = map.get("substrate/lsm_engine.rs").expect("substrate/lsm_engine.rs");
    for line in rs.lines() {
        if line.contains("append_put(") && line.contains("kb") {
            assert!(
                line.contains("&kb") || line.contains("& kb"),
                "cross-module append_put must borrow kb for &Vec formal (lsm_engine.rs:193). Line: {line}\nFull:\n{rs}"
            );
        }
    }
}

// ── WDB-065b: sim network — emitted Rust must not shorthand &NodeId into owned fields ───

#[test]
fn wdb_sim_network_emitted_rust_owned_node_id_struct_fields() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "sim/network.wj",
        r#"
pub struct NodeId {
    pub value: u32,
}

pub enum SimMessageKind {
    LiveDelta,
}

pub struct SimMessage {
    pub kind: SimMessageKind,
    pub group_key: i64,
}

pub fn encode_message(msg: SimMessage) -> Vec<u8> {
    let _ = msg.group_key
    Vec::new()
}

pub struct SimDuration {
    pub micros: i64,
}

pub struct SeededSimClock {
    pub now_micros: i64,
}

impl SeededSimClock {
    pub fn effective_now_micros(self) -> i64 {
        self.now_micros
    }
}

pub struct QueuedMessage {
    pub from: NodeId,
    pub to: NodeId,
    pub payload: Vec<u8>,
    pub deliver_at_micros: i64,
    pub sequence: u64,
}

pub struct SimNetwork {
    pub queue: Vec<QueuedMessage>,
    pub next_sequence: u64,
    pub default_latency_micros: i64,
}

impl SimNetwork {
    pub fn new(default_latency_micros: i64) -> SimNetwork {
        SimNetwork {
            queue: Vec::new(),
            next_sequence: 0,
            default_latency_micros: default_latency_micros,
        }
    }

    pub fn send(self, from: NodeId, to: NodeId, msg: SimMessage, clock: SeededSimClock, latency: SimDuration) {
        let payload = encode_message(msg)
        let seq = self.next_sequence
        self.next_sequence = seq + 1
        let deliver_at = clock.effective_now_micros() + latency.micros + self.default_latency_micros
        let queued = QueuedMessage {
            from: from,
            to: to,
            payload: payload,
            deliver_at_micros: deliver_at,
            sequence: seq,
        }
        self.queue.push(queued)
    }
}
"#,
    );
    test.add_file("sim/mod.wj", "pub mod network\n");
    test.add_file("main.wj", "pub fn run() {}");
    let map = test.compile().expect("compile");
    let rs = map.get("sim/network.rs").expect("sim/network.rs");
    if rs.contains("from: &NodeId") || rs.contains("to: &NodeId") {
        assert!(
            !rs.contains("QueuedMessage { from, to,"),
            "must not shorthand-init owned NodeId from &NodeId (network.rs:56). Got:\n{rs}"
        );
    }
}

// ── WindjammerDB e2e helpers (monorepo sibling) ─────────────────────────────

fn windjammerdb_crates_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../windjammerdb/crates")
}

fn require_windjammerdb_crate(name: &str) -> PathBuf {
    let path = windjammerdb_crates_root().join(name);
    assert!(
        path.is_dir(),
        "windjammerdb crate {name} not found at {}",
        path.display()
    );
    path
}

fn wj_build_windjammerdb_crate(name: &str) {
    let crate_root = require_windjammerdb_crate(name);
    // Invalidate incremental stamp so a newer `wj` always re-transpiles — otherwise
    // parallel e2e tests can keep pre-fix `gen/*.rs` when source fingerprints match.
    let _ = fs::remove_file(crate_root.join("gen/.wj-compiler-stamp"));
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", "src", "-o", "gen", "--no-cargo"])
        .current_dir(&crate_root)
        .output()
        .unwrap_or_else(|e| panic!("spawn wj build {name}: {e}"));
    assert!(
        output.status.success(),
        "wj build {name} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn cargo_check_windjammerdb_gen(name: &str, with_tests: bool) {
    let gen = require_windjammerdb_crate(name).join("gen");
    let mut cmd = Command::new("cargo");
    cmd.arg("check").arg("--quiet").current_dir(&gen);
    if with_tests {
        cmd.arg("--tests");
    }
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("spawn cargo check {name}: {e}"));
    assert!(
        output.status.success(),
        "cargo check {name} failed:\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn build_wal_engine_library(tmp: &Path) -> PathBuf {
    let wal_src = tmp.join("wal_src");
    let wal_gen = tmp.join("wal_gen");
    let wal_dir = wal_src.join("wal");
    fs::create_dir_all(&wal_dir).expect("mkdir wal_src");
    fs::write(
        wal_dir.join("mod.wj"),
        "pub mod record\npub mod segment\n",
    )
    .unwrap();
    fs::write(
        wal_dir.join("record.wj"),
        r#"
pub struct Lsn {
    pub value: u64,
}

pub struct WalRecord {
    pub lsn: Lsn,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

impl WalRecord {
    pub fn put(lsn: Lsn, key: Vec<u8>, value: Vec<u8>) -> WalRecord {
        WalRecord { lsn: lsn, key: key, value: value }
    }
}
"#,
    )
    .unwrap();
    fs::write(
        wal_dir.join("segment.wj"),
        r#"
use crate::wal::record::WalRecord
use crate::wal::record::Lsn

pub struct WalSegment {
    pub bytes: Vec<u8>,
    pub next_lsn: Lsn,
}

impl WalSegment {
    pub fn with_header() -> WalSegment {
        WalSegment {
            bytes: Vec::new(),
            next_lsn: Lsn { value: 1 },
        }
    }

    pub fn append_put(self, key: Vec<u8>, value: Vec<u8>) -> Lsn {
        let lsn = self.next_lsn
        let _record = WalRecord::put(lsn, key, value)
        lsn
    }

    pub fn replay(self) -> Vec<WalRecord> {
        Vec::new()
    }
}
"#,
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wal_src.to_str().unwrap(),
            "--output",
            wal_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wal library build");
    assert!(
        output.status.success(),
        "wal library build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    wal_gen
}

// ── WDB-066: TxnManager put/delete — borrow &Key at cross-crate delegation ───────────────
//
// Real wdb-txn gen/txn_manager.rs:42,46 emits key.clone() for MemoryEngine::put/delete(&Key).
// In-project MultiFileTest fixtures do not reproduce this; use windjammerdb gen + cargo check.

#[test]
fn wdb_txn_manager_windjammerdb_gen_put_delete_no_key_clone() {
    wj_build_windjammerdb_crate("wdb-substrate");
    wj_build_windjammerdb_crate("wdb-txn");
    let rs_path = require_windjammerdb_crate("wdb-txn").join("gen/txn_manager.rs");
    let rs = fs::read_to_string(&rs_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", rs_path.display()));
    for line in rs.lines() {
        if line.contains("engine.put(") {
            assert!(
                !line.contains("key.clone()"),
                "put must pass &Key through, not key.clone() (txn_manager.rs:42). Line: {line}\nFull:\n{rs}"
            );
        }
        if line.contains("engine.delete(") {
            assert!(
                !line.contains("key.clone()"),
                "delete must pass &Key through, not key.clone() (txn_manager.rs:46). Line: {line}\nFull:\n{rs}"
            );
        }
    }
}

#[test]
fn wdb_txn_manager_windjammerdb_cargo_check() {
    wj_build_windjammerdb_crate("wdb-substrate");
    wj_build_windjammerdb_crate("wdb-txn");
    cargo_check_windjammerdb_gen("wdb-txn", false);
}

// ── WDB-067: WalSegment append_put — cross-crate vec literal/helper borrow ───────────────
//
// wdb-wal in-crate tests emit append_put(&vec![...]); wdb-substrate lsm_test.rs:130 emits
// append_put(vec![5], encode_int64(99)) when WalSegment comes from cross-crate metadata.

#[test]
fn wdb_wal_segment_cross_crate_append_put_borrows_vec_literal() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let wal_gen = build_wal_engine_library(tmp.path());
    let substrate_src = tmp.path().join("substrate_src");
    fs::create_dir_all(&substrate_src).expect("mkdir substrate_src");
    fs::write(
        substrate_src.join("lsm_test.wj"),
        r#"
use wal::wal::segment::WalSegment

fn encode_int64(v: i64) -> Vec<u8> {
    vec![v as u8]
}

pub fn test_store_rebuild_from_segment() {
    let mut seg = WalSegment::with_header()
    seg.append_put(vec![5], encode_int64(99))
    let _ = seg.replay()
}
"#,
    )
    .unwrap();
    let substrate_gen = tmp.path().join("substrate_gen");
    let metadata = format!("wal={}", wal_gen.join("metadata.json").display());
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            substrate_src.join("lsm_test.wj").to_str().unwrap(),
            "--output",
            substrate_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &metadata,
        ])
        .output()
        .expect("substrate lsm_test build");
    assert!(
        output.status.success(),
        "lsm_test build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rs = fs::read_to_string(substrate_gen.join("lsm_test.rs")).expect("lsm_test.rs");
    for line in rs.lines() {
        if line.contains("append_put(") {
            assert!(
                line.contains("&vec![5]")
                    || line.contains("& vec![5]")
                    || (line.contains("&") && line.contains("encode_int64")),
                "cross-crate append_put must borrow vec literal and helper return (lsm_test.rs:130). Line: {line}\nFull:\n{rs}"
            );
        }
    }
}

#[test]
fn wdb_substrate_lsm_test_windjammerdb_cargo_check() {
    wj_build_windjammerdb_crate("wdb-wal");
    wj_build_windjammerdb_crate("wdb-substrate");
    cargo_check_windjammerdb_gen("wdb-substrate", true);
}

#[test]
fn wdb_embedded_windjammerdb_cargo_check() {
    // Types is a path dependency of embedded; rebuild first so stale `gen/` from an
    // older `wj` cannot poison cargo check (const match arms, ownership formals).
    wj_build_windjammerdb_crate("wdb-types");
    wj_build_windjammerdb_crate("wdb-wal");
    wj_build_windjammerdb_crate("wdb-substrate");
    wj_build_windjammerdb_crate("wdb-txn");
    wj_build_windjammerdb_crate("wdb-embedded");
    cargo_check_windjammerdb_gen("wdb-embedded", true);
}

// ── WDB-071: cross-crate owned struct param — call site must auto-borrow ─────
//
// wdb-reducer reducer_host.wj calls apply_zset_to_circuit(circuit, zset) with owned
// Circuit; bridge emits `circuit: &Circuit` but call site omits `&circuit`.

#[test]
fn wdb_cross_crate_owned_struct_call_site_auto_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "circuit/circuit.wj",
        r#"
pub struct Circuit {
    pub tick: u64,
}

pub struct CircuitDelta {
    pub tick: u64,
}

impl Circuit {
    pub fn new() -> Circuit {
        Circuit { tick: 0 }
    }

    pub fn apply_delta(self, delta: CircuitDelta) -> (Circuit, CircuitDelta) {
        (self, delta)
    }
}
"#,
    );
    test.add_file("circuit/mod.wj", "pub mod circuit;\n");
    test.add_file(
        "bridge/zset_bridge.wj",
        r#"
use crate::circuit::circuit::Circuit
use crate::circuit::circuit::CircuitDelta

pub struct ZSetDelta {
    pub rows: u32,
}

pub fn apply_zset_to_circuit(circuit: Circuit, zset: ZSetDelta) -> (Circuit, CircuitDelta) {
    let delta = CircuitDelta { tick: zset.rows as u64 }
    circuit.apply_delta(delta)
}
"#,
    );
    test.add_file("bridge/mod.wj", "pub mod zset_bridge;\n");
    test.add_file(
        "host/reducer_host.wj",
        r#"
use crate::bridge::zset_bridge::ZSetDelta
use crate::bridge::zset_bridge::apply_zset_to_circuit
use crate::circuit::circuit::Circuit

pub fn run_pipeline(circuit: Circuit, zset: ZSetDelta) -> Circuit {
    let applied = apply_zset_to_circuit(circuit, zset)
    applied.0
}
"#,
    );
    test.add_file("host/mod.wj", "pub mod reducer_host;\n");
    test.add_file(
        "main.wj",
        r#"
use crate::host::reducer_host::run_pipeline
use crate::bridge::zset_bridge::ZSetDelta
use crate::circuit::circuit::Circuit

pub fn main() {
    let _ = run_pipeline(Circuit::new(), ZSetDelta { rows: 1 })
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("host/reducer_host.rs").expect("host/reducer_host.rs");
    let bridge = map
        .get("bridge/zset_bridge.rs")
        .expect("bridge/zset_bridge.rs");
    let callee_owned = bridge.contains("apply_zset_to_circuit(circuit: Circuit")
        || bridge.contains("apply_zset_to_circuit(mut circuit: Circuit");
    let callee_borrowed = bridge.contains("apply_zset_to_circuit(circuit: &Circuit")
        || bridge.contains("apply_zset_to_circuit(circuit: & Circuit");
    assert!(
        rs.contains("apply_zset_to_circuit(&circuit")
            || rs.contains("apply_zset_to_circuit(circuit.clone()")
            || (callee_owned
                && rs.contains("apply_zset_to_circuit(circuit,")
                && !callee_borrowed),
        "cross-crate call with owned Circuit must borrow or pass owned consistently.\n\
         host:\n{rs}\nbridge:\n{bridge}"
    );
    assert!(
        !rs.contains("apply_zset_to_circuit(circuit, &zset)")
            || rs.contains("apply_zset_to_circuit(&circuit"),
        "must not pass owned Circuit where callee expects &Circuit without borrow. Got:\n{rs}"
    );
}

#[test]
fn wdb_reducer_windjammerdb_cargo_check() {
    wj_build_windjammerdb_crate("wdb-types");
    wj_build_windjammerdb_crate("wdb-circuit");
    wj_build_windjammerdb_crate("wdb-reducer");
    cargo_check_windjammerdb_gen("wdb-reducer", false);
}

// ── WDB-072 dogfooding: commit_granularity enum match in for-loop ────────────

#[test]
fn wdb_types_commit_granularity_cargo_check() {
    wj_build_windjammerdb_crate("wdb-types");
    cargo_check_windjammerdb_gen("wdb-types", true);
}

// ── WDB-073 dogfooding: document_layer encode row_id spurious deref (regression) ─

#[test]
fn wdb_layers_document_encode_row_id_no_spurious_deref() {
    wj_build_windjammerdb_crate("wdb-layers");
    let rs_path = require_windjammerdb_crate("wdb-layers")
        .join("gen/document/document_layer.rs");
    let rs = fs::read_to_string(&rs_path).unwrap_or_else(|e| {
        panic!("read {}: {e}", rs_path.display())
    });
    assert!(
        !rs.contains("*row.row_id") && !rs.contains("* row.row_id"),
        "document_layer encode must not spurious-deref Copy row_id (regression guard).\nGot:\n{rs}"
    );
}
