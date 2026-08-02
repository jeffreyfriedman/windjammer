#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! Failing regressions for **remaining compiler workarounds**.
//!
//! Each test mirrors a shim documented in `dogfood workaround notes`.
//! Fix in `windjammer/` → revert the corresponding shim → test passes.
//!
//! Run: `cargo test --release --test all ownership_workaround_regression`

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

// ── authz regression: RebacTupleStore method returning (Self, T) ─────────────────
//
// dogfood workaround: `store_has_tuple` / `store_list_subjects` free fns
// (rebac_tuple_store.wj) because `has_tuple` → `(RebacTupleStore, bool)` on impl
// breaks ownership codegen at recursive resolver call sites.

#[test]
fn dogfood_authz_owned_store_method_returns_self_and_bool() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "authz/subject.wj",
        r#"
pub struct SubjectRef {
    pub subject_type: string,
    pub subject_id: string,
}
"#,
    );
    test.add_file(
        "authz/object.wj",
        r#"
pub struct ObjectRef {
    pub object_type: string,
    pub object_id: string,
}
"#,
    );
    test.add_file(
        "authz/store.wj",
        r#"
use crate::authz::object::ObjectRef
use crate::authz::subject::SubjectRef

pub struct TupleStore {
    count: u32,
}

impl TupleStore {
    pub fn new() -> TupleStore {
        TupleStore { count: 0 }
    }

    pub fn has_tuple(
        self,
        subject: SubjectRef,
        relation: string,
        object: ObjectRef,
    ) -> (TupleStore, bool) {
        let _ = (subject, relation, object)
        (self, self.count > 0)
    }
}
"#,
    );
    test.add_file(
        "authz/resolver.wj",
        r#"
use crate::authz::object::ObjectRef
use crate::authz::store::TupleStore
use crate::authz::subject::SubjectRef

pub fn resolve_check_store(
    store: TupleStore,
    subject: SubjectRef,
    relation: string,
    object: ObjectRef,
    depth: u32,
) -> bool {
    let (s, direct) = store.has_tuple(subject, relation, object)
    if direct {
        return true
    }
    if depth > 2 {
        return false
    }
    let (s2, nested) = s.has_tuple(subject, relation, object)
    let _ = s2
    nested
}
"#,
    );
    test.add_file(
        "authz/mod.wj",
        "pub mod subject\npub mod object\npub mod store\npub mod resolver\n",
    );
    test.add_file(
        "main.wj",
        r#"
use crate::authz::object::ObjectRef
use crate::authz::resolver::resolve_check_store
use crate::authz::store::TupleStore
use crate::authz::subject::SubjectRef

pub fn run() -> bool {
    let store = TupleStore::new()
    resolve_check_store(
        store,
        SubjectRef { subject_type: "user", subject_id: "alice" },
        "viewer",
        ObjectRef { object_type: "doc", object_id: "1" },
        0,
    )
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("authz/resolver.rs").expect("authz/resolver.rs");
    for line in rs.lines() {
        if line.contains("has_tuple(") {
            assert!(
                !line.contains(".clone()"),
                "owned store threaded through (Self, bool) method must not clone at call site.\nLine: {line}\nFull:\n{rs}"
            );
        }
    }
}

// ── regression (exact layout): harness drain_network extract-assign ─────────────
//
// dogfood workaround: `let mut net = self.network` in harness.wj drain_network.
// Generated harness.rs still emits `self.network.clone().poll()` — extract does not help.

#[test]
fn dogfood_harness_exact_drain_network_extract_no_clone() {
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
        "sim/message.wj",
        r#"
pub struct SimMessage {
    pub payload: int,
}
"#,
    );
    test.add_file(
        "sim/network.wj",
        r#"
use crate::sim::clock::LogicalTime
use crate::sim::message::SimMessage

pub struct SimNetwork {
    pending: Vec<SimMessage>,
}

impl SimNetwork {
    pub fn new() -> SimNetwork {
        SimNetwork { pending: Vec::new() }
    }

    pub fn poll(self, clock: LogicalTime) -> Option<SimMessage> {
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
        "sim/view.wj",
        r#"
use crate::sim::message::SimMessage

pub struct PartialAggregateView {
    count: int,
}

impl PartialAggregateView {
    pub fn new() -> PartialAggregateView {
        PartialAggregateView { count: 0 }
    }

    pub fn enqueue_update(self, msg: SimMessage) -> PartialAggregateView {
        let _ = msg.payload
        PartialAggregateView { count: self.count + 1 }
    }
}
"#,
    );
    test.add_file(
        "sim/harness.wj",
        r#"
use crate::sim::clock::LogicalTime
use crate::sim::network::SimNetwork
use crate::sim::view::PartialAggregateView

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

    pub fn tick(self) {
        let _ = self.clock.tick
    }

    pub fn drain_network(self, view: PartialAggregateView) -> PartialAggregateView {
        let mut current = view
        loop {
            let mut net = self.network
            let polled = net.poll(self.clock)
            self.network = net
            match polled {
                Some(msg) => {
                    current = current.enqueue_update(msg)
                    self.tick()
                }
                None => {
                    break
                }
            }
        }
        current
    }
}
"#,
    );
    test.add_file(
        "sim/mod.wj",
        "pub mod clock\npub mod message\npub mod network\npub mod view\npub mod harness\n",
    );
    test.add_file(
        "main.wj",
        r#"
use crate::sim::harness::SimHarness
use crate::sim::view::PartialAggregateView

pub fn run() -> int {
    SimHarness::new().drain_network(PartialAggregateView::new()).count
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("sim/harness.rs").expect("sim/harness.rs");
    assert!(
        !rs.contains("network.clone()") && !rs.contains("self.network.clone()"),
        "drain_network must not clone network field (regression). Got:\n{rs}"
    );
    assert!(
        !rs.contains("let mut net = self.network") || rs.contains("net.poll("),
        "extract-assign pattern should compile to in-place poll, not clone. Got:\n{rs}"
    );
}

// ── regression (delete path): apply_patch_delete must not re-box key via copy ───
//
// dogfood workaround: `Key::new(copy_key_bytes(key))` in store.wj apply_patch_delete
// when base_parts exist (patch path).

#[test]
fn dogfood_lsm_apply_patch_delete_passes_owned_key_directly() {
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
        "types/part_id.wj",
        r#"
pub struct PartId {
    pub value: u64,
}

impl PartId {
    pub fn new(value: u64) -> PartId {
        PartId { value: value }
    }
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\npub mod part_id\n");
    test.add_file(
        "lsm/hot.wj",
        r#"
use crate::types::key::Key

pub struct HotPart {
    keys: Vec<Key>,
}

impl HotPart {
    pub fn new() -> HotPart {
        HotPart { keys: Vec::new() }
    }

    pub fn delete_key(self, key: Key) {
        let _ = key.bytes.len()
    }
}
"#,
    );
    test.add_file(
        "lsm/base.wj",
        r#"
use crate::types::key::Key
use crate::types::part_id::PartId

pub struct BasePart {
    pub part_id: PartId,
    pub keys: Vec<Key>,
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
"#,
    );
    test.add_file(
        "lsm/patch.wj",
        r#"
use crate::types::key::Key
use crate::types::part_id::PartId

pub struct PatchPart {
    pub base_part_id: PartId,
}

impl PatchPart {
    pub fn new(base_part_id: PartId) -> PatchPart {
        PatchPart { base_part_id: base_part_id }
    }

    pub fn apply_delete(self, key: Key) {
        let _ = key.bytes.len()
    }
}
"#,
    );
    test.add_file(
        "lsm/store.wj",
        r#"
use crate::lsm::base::BasePart
use crate::lsm::hot::HotPart
use crate::lsm::patch::PatchPart
use crate::types::key::Key
use crate::types::part_id::PartId

pub struct LsmStore {
    hot: HotPart,
    base_parts: Vec<BasePart>,
    patches: Vec<PatchPart>,
}

impl LsmStore {
    pub fn new() -> LsmStore {
        LsmStore {
            hot: HotPart::new(),
            base_parts: Vec::new(),
            patches: Vec::new(),
        }
    }

    pub fn seed_base(self) -> LsmStore {
        let base = BasePart {
            part_id: PartId::new(1),
            keys: Vec::new(),
        }
        self.base_parts.push(base)
        self
    }

    pub fn delete_key(self, key: Key) {
        self.apply_patch_delete(key)
    }

    fn key_in_latest_base(self, key: Key) -> bool {
        let base_count = self.base_parts.len()
        if base_count == 0 {
            false
        } else {
            let latest = self.base_parts[base_count - 1]
            latest.has_key(key)
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
                let patch_base = self.patches[patch_idx].base_part_id
                if patch_base.value == base_id.value {
                    self.patches[patch_idx].apply_delete(key)
                } else {
                    let mut patch = PatchPart::new(base_id)
                    patch.apply_delete(key)
                    self.patches.push(patch)
                }
            }
        }
    }
}
"#,
    );
    test.add_file("lsm/mod.wj", "pub mod hot\npub mod base\npub mod patch\npub mod store\n");
    test.add_file(
        "main.wj",
        r#"
use crate::lsm::store::LsmStore
use crate::types::key::Key

pub fn run() {
    let store = LsmStore::new().seed_base()
    store.delete_key(Key { bytes: vec![1u8] })
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("lsm/store.rs").expect("lsm/store.rs");
    assert!(
        !rs.contains("copy_key_bytes") && !rs.contains("Key::new(copy_key"),
        "apply_patch_delete with base_parts must forward owned Key without copy_key_bytes re-box.\nGot:\n{rs}"
    );
}

// ── regression/041: BTreeIndex should expose `get`, not renamed `lookup` ────────
//
// dogfood workaround: method renamed `lookup` because `get` triggered borrow bugs.

#[test]
fn dogfood_btree_index_get_method_name_and_owned_key() {
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
    Int64(i64),
}
"#,
    );
    test.add_file("types/mod.wj", "pub mod key\npub mod value\n");
    test.add_file(
        "index/btree.wj",
        r#"
use crate::types::key::Key
use crate::types::value::Value

pub struct BTreeIndex {
    entries: Vec<(Key, Value)>,
}

impl BTreeIndex {
    pub fn new() -> BTreeIndex {
        BTreeIndex { entries: Vec::new() }
    }

    pub fn get(self, key: Key) -> Option<Value> {
        for entry in self.entries {
            if entry.0.bytes == key.bytes {
                return Some(entry.1)
            }
        }
        None
    }
}
"#,
    );
    test.add_file("index/mod.wj", "pub mod btree\n");
    test.add_file(
        "main.wj",
        r#"
use crate::index::btree::BTreeIndex
use crate::types::key::Key

fn key_ten() -> Key {
    Key { bytes: vec![10u8] }
}

pub fn lookup() -> bool {
    let index = BTreeIndex::new()
    match index.get(key_ten()) {
        Some(_) => true,
        None => false,
    }
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("main.rs").expect("main.rs");
    assert!(
        rs.contains(".get(") && !rs.contains(".lookup("),
        "canonical index API is get(key), not lookup rename workaround. Got:\n{rs}"
    );
    assert!(
        !rs.contains("key_ten().clone()") && !rs.contains("key.clone()"),
        "owned helper-return Key must pass to get without clone. Got:\n{rs}"
    );
}

// ── regression: enum variant fields in for-loop match are borrowed refs ─────────
//
// Dogfood regression: commit_granularity.wj FlushTrigger::should_flush
// `for condition in conditions { match condition { RowCount { threshold } => row_count >= threshold }`
// rustc: expected `u64`, found `&u64` on threshold comparisons.

#[test]
fn dogfood_enum_match_for_loop_borrowed_variant_fields() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "flush.wj",
        r#"
pub enum FlushCondition {
    RowCount { threshold: u64 },
    IdleMs { threshold: u64 },
}

pub enum FlushTrigger {
    Any { conditions: Vec<FlushCondition> },
    Explicit,
}

impl FlushTrigger {
    pub fn should_flush(self, row_count: u64, idle_ms: u64) -> bool {
        match self {
            FlushTrigger::Explicit => false,
            FlushTrigger::Any { conditions } => {
                for condition in conditions {
                    match condition {
                        FlushCondition::RowCount { threshold } => {
                            if row_count >= threshold {
                                return true
                            }
                        }
                        FlushCondition::IdleMs { threshold } => {
                            if idle_ms >= threshold {
                                return true
                            }
                        }
                    }
                }
                false
            }
        }
    }
}
"#,
    );
    test.add_file(
        "main.wj",
        r#"
use crate::flush::FlushCondition
use crate::flush::FlushTrigger

pub fn run() -> bool {
    let mut conditions = Vec::new()
    conditions.push(FlushCondition::RowCount { threshold: 10 })
    FlushTrigger::Any { conditions: conditions }.should_flush(11, 0)
}
"#,
    );

    test.assert_compiles_without_error();
}

// ── regression: Copy struct field via borrowed row in for-loop — spurious deref ─
//
// Dogfood regression: document_layer.wj encode(row) in multipass crate build
// Generated Rust: `self.field_key(*row.row_id, ...)` → E0614 on i64.

#[test]
fn dogfood_copy_field_via_borrowed_struct_no_spurious_deref() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "data_model_layer.wj",
        r#"
pub struct ColumnCell {
    pub ordinal: u32,
    pub value: u32,
}

pub struct LogicalRow {
    pub row_id: i64,
    pub columns: Vec<ColumnCell>,
}

pub struct LayerCircuitRef {
    pub table_id: u32,
    pub source_node_id: u32,
}
"#,
    );
    test.add_file(
        "document_layer.wj",
        r#"
use crate::data_model_layer::ColumnCell
use crate::data_model_layer::LogicalRow
use crate::data_model_layer::LayerCircuitRef

pub struct DocumentLayer {
    pub collection_id: u32,
}

impl DocumentLayer {
    pub fn new(collection_id: u32) -> DocumentLayer {
        DocumentLayer { collection_id: collection_id }
    }

    pub fn encode(self, row: LogicalRow) -> u32 {
        let mut out_len = 0
        for cell in row.columns {
            let _ = self.field_key(row.row_id, cell.ordinal)
            out_len = out_len + 1
        }
        out_len
    }

    pub fn circuit_source(self) -> LayerCircuitRef {
        LayerCircuitRef {
            table_id: self.collection_id,
            source_node_id: 2,
        }
    }

    fn field_key(self, doc_id: i64, field_ordinal: u32) -> i64 {
        doc_id + field_ordinal as i64
    }
}
"#,
    );
    test.add_file(
        "main.wj",
        r#"
use crate::data_model_layer::ColumnCell
use crate::data_model_layer::LogicalRow
use crate::document_layer::DocumentLayer

pub fn run() -> i64 {
    let row = LogicalRow {
        row_id: 42,
        columns: vec![ColumnCell { ordinal: 1, value: 7 }],
    }
    DocumentLayer::new(3).encode(row) as i64
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("document_layer.rs").expect("document_layer.rs");
    assert!(
        !rs.contains("*row.row_id") && !rs.contains("* row.row_id"),
        "multipass encode must not spurious-deref Copy row_id field. Got:\n{rs}"
    );
    test.assert_compiles_without_error();
}

// ── regression: method returning tuple — `.0`/`.1` field access ────────────────
//
// Dogfood regression: trace_ingestion.wj complete_trace() → outcome.1.kept

#[test]
fn dogfood_tuple_return_dot_access_compiles() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "trace.wj",
        r#"
pub struct TraceOutcome {
    pub kept: bool,
    pub span_count: u32,
}

pub struct TraceBuffer {
    pub spans: u32,
}

impl TraceBuffer {
    pub fn new() -> TraceBuffer {
        TraceBuffer { spans: 0 }
    }

    pub fn ingest(self) -> TraceBuffer {
        TraceBuffer { spans: self.spans + 1 }
    }

    pub fn complete(self) -> (TraceBuffer, TraceOutcome) {
        let outcome = TraceOutcome {
            kept: self.spans > 0,
            span_count: self.spans,
        }
        (TraceBuffer { spans: 0 }, outcome)
    }
}
"#,
    );
    test.add_file(
        "main.wj",
        r#"
use crate::trace::TraceBuffer

pub fn run() -> bool {
    let mut buffer = TraceBuffer::new()
    buffer = buffer.ingest()
    let outcome = buffer.complete()
    outcome.1.kept && outcome.0.spans == 0
}
"#,
    );

    test.assert_compiles_without_error();
}

// ── regression: cross-crate Key::new(encode_key(parts)) owned Vec passthrough ─
//
// Dogfood regression: layer crates call types_crate::Key::new(encode_key(parts)).

#[test]
fn dogfood_cross_crate_key_new_encode_key_owned_vec() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "key.wj",
        r#"
pub struct Key {
    pub bytes: Vec<u8>,
}

impl Key {
    pub fn new(bytes: Vec<u8>) -> Key {
        Key { bytes: bytes }
    }
}

pub fn encode_key(parts: Vec<u8>) -> Vec<u8> {
    parts
}
"#,
    );
    test.add_file(
        "encode.wj",
        r#"
use crate::key::Key
use crate::key::encode_key

pub fn row_key(table_id: u32, row_id: i64) -> Key {
    let mut parts = Vec::new()
    parts.push(table_id as u8)
    parts.push(row_id as u8)
    Key::new(encode_key(parts))
}
"#,
    );
    test.add_file(
        "main.wj",
        r#"
use crate::encode::row_key

pub fn run() -> u32 {
    row_key(1, 2).bytes.len() as u32
}
"#,
    );

    test.assert_compiles_without_error();
}

// ── regression: same-crate string wrapper — owned literal to extern fn ────────
//
// Dogfood regression: mcp_adapter.wj mcp_build_tool_result("dogfood_query", count)

#[test]
fn dogfood_same_crate_string_wrapper_owned_literal() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "rpc.wj",
        r#"
extern fn build_result_ffi(tool_name: string, row_count: u64) -> string

pub fn build_result(tool_name: string, row_count: u64) -> string {
    build_result_ffi(tool_name, row_count)
}
"#,
    );
    test.add_file(
        "adapter.wj",
        r#"
use crate::rpc::build_result

pub struct QueryResult {
    pub row_count: u32,
}

pub struct Adapter {
    pub name: string,
}

impl Adapter {
    pub fn encode(self, result: QueryResult) -> string {
        build_result("dogfood_query", result.row_count as u64)
    }
}
"#,
    );
    test.add_file(
        "main.wj",
        r#"
use crate::adapter::Adapter
use crate::adapter::QueryResult

pub fn run() -> string {
    Adapter { name: "external dogfood" }.encode(QueryResult { row_count: 3 })
}
"#,
    );

    test.assert_compiles_without_error();
}
