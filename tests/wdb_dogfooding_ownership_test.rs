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
//! ## Open compiler issues — expected suite state (Jul 23 PM, wj 0.50.0)
//!
//! | ID | Primary failing test(s) | Notes |
//! |----|-------------------------|-------|
//! | WDB-047 | `wdb_substrate_full_store_patch_part_enum_value_layout` | **FAIL** — real `wdb-substrate/store.wj` layout (enum `Value`, `PatchPart`) |
//! | WDB-050 | `wdb_wal_segment_vec_literal_borrow_at_append` | **FAIL** — `append_put(vec![…])` must borrow for `&Vec<u8>` formals |
//! | WDB-046/047 | `wdb_lsm_store_apply_patch_*`, `wdb_store_has_key_*` | pass (minimal fixture fixed) |
//! | WDB-049 | `wdb_wal_ffi_snapshot_and_path_borrow_at_call_sites` | pass (writer/replay paths fixed) |
//!
//! **Expected today:** 22 pass, **2 fail** until full substrate + wal segment literal fixes land.
//!
//! Closed-issue regressions (must stay passing): WDB-044, WDB-045, WDB-048.
//! Guard tests (workaround removal pending full build): WDB-019, WDB-039, WDB-042.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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
    assert!(
        rs.contains(r#""users".to_string()"#) || rs.contains(r#""users".to_string("#),
        "cross-crate string literal must coerce to owned String. Got:\n{rs}"
    );
    assert!(
        !rs.contains(r#"register_table("users""#) || rs.contains(".to_string()"),
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
        rs.contains("pub fn put_value(&self, key: Key") || rs.contains("pub fn put_value(&mut self, key: Key"),
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
        let _ = key.bytes.len()
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
        let _ = key.bytes.len()
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
