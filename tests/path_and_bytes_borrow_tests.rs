//! Auto-borrow owned values at call sites when the callee formal is inferred `&T`.
//!
//! **Run:**
//! ```bash
//! cd windjammer && unset CARGO_TARGET_DIR && cargo test --release --test all path_bytes_
//! ```
//!
//! **Fix target:** `windjammer/src/codegen/rust/` — call-site borrow insertion when resolved
//! callee signature has `param_ownership[i] == Borrowed` but argument is owned (field access,
//! FFI return, or parameter forward).
//!
//! Also covered in `cross_crate_dogfooding_ownership_test.rs`
//! (`dogfood_wal_ffi_snapshot_and_path_borrow_at_call_sites`, etc.).
//!
//! Optional e2e: set `WJ_DOGFOOD_CRATES_DIR` to enable external crate cargo-check gates.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

// ── gate-a: FFI Vec return → &Vec formal at struct ctor ─────────────────────────────

#[test]
fn path_bytes_ffi_vec_return_borrowed_at_from_bytes() {
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
    pub fn from_bytes(bytes: Vec<u8>) -> WalSegment {
        WalSegment { bytes: bytes }
    }
}
"#,
    );
    test.add_file(
        "wal/writer.wj",
        r#"
use crate::wal::ffi::wal_snapshot_load_ffi
use crate::wal::segment::WalSegment

pub struct WalWriter {
    path: string,
    segment: WalSegment,
}

impl WalWriter {
    pub fn open_existing(path: string) -> WalWriter {
        let snapshot = wal_snapshot_load_ffi(path)
        let segment = WalSegment::from_bytes(snapshot)
        WalWriter { path: path, segment: segment }
    }
}
"#,
    );
    test.add_file("wal/mod.wj", "pub mod ffi\npub mod segment\npub mod writer\n");
    test.add_file(
        "main.wj",
        r#"
use crate::wal::writer::WalWriter

pub fn run() {
    let _ = WalWriter::open_existing("wal.log")
}
"#,
    );

    let map = test.compile().expect("compile");
    let seg = map.get("wal/segment.rs").expect("wal/segment.rs");
    eprintln!("SEGMENT:\n{seg}");
    let rs = map.get("wal/writer.rs").expect("wal/writer.rs");
    assert!(
        rs.contains("from_bytes(&snapshot)")
            || rs.contains("from_bytes(& snapshot)"),
        "gate-a: FFI Vec snapshot must borrow at from_bytes(&Vec) call site. Got:\n{rs}"
    );
    assert!(
        !rs.contains("from_bytes(snapshot)") || rs.contains("&snapshot"),
        "gate-a: must not move owned snapshot into &Vec formal. Got:\n{rs}"
    );
}

// ── gate-b: self.path string field → &str formal at free fn call ─────────────────────

#[test]
fn path_bytes_path_field_borrowed_at_replay_all() {
    let mut test = MultiFileTest::new();
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
"#,
    );
    test.add_file(
        "wal/writer.wj",
        r#"
use crate::wal::record::WalRecord
use crate::wal::replay::replay_all

pub struct WalWriter {
    path: string,
}

impl WalWriter {
    pub fn replay_all_records(self) -> Vec<WalRecord> {
        replay_all(self.path)
    }
}
"#,
    );
    test.add_file("wal/mod.wj", "pub mod record\npub mod replay\npub mod writer\n");
    test.add_file(
        "main.wj",
        r#"
use crate::wal::writer::WalWriter

pub fn run() {
    let writer = WalWriter { path: "wal.log" }
    let _ = writer.replay_all_records()
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("wal/writer.rs").expect("wal/writer.rs");
    assert!(
        rs.contains("replay_all(&self.path)")
            || rs.contains("replay_all(self.path.as_str())")
            || rs.contains("replay_all(& self.path)"),
        "gate-b: self.path must borrow as &str for replay_all(path: &str). Got:\n{rs}"
    );
    assert!(
        !rs.contains("replay_all(self.path.clone())"),
        "gate-b: must not clone String for &str formal. Got:\n{rs}"
    );
}

// ── gate-c: self.path + Copy Lsn at replay_to_lsn ────────────────────────────────────

#[test]
fn path_bytes_path_field_borrowed_at_replay_to_lsn() {
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
}
"#,
    );
    test.add_file(
        "wal/replay.wj",
        r#"
use crate::wal::record::WalRecord
use crate::wal::lsn::Lsn

pub fn replay_to_lsn(path: string, through: Lsn) -> Vec<WalRecord> {
    let _ = (path, through)
    Vec::new()
}
"#,
    );
    test.add_file(
        "wal/writer.wj",
        r#"
use crate::wal::lsn::Lsn
use crate::wal::record::WalRecord
use crate::wal::replay::replay_to_lsn

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
    test.add_file("wal/mod.wj", "pub mod lsn\npub mod record\npub mod replay\npub mod writer\n");
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
    assert!(
        rs.contains("replay_to_lsn(&self.path")
            || rs.contains("replay_to_lsn(self.path.as_str()")
            || rs.contains("replay_to_lsn(& self.path"),
        "gate-c: self.path must borrow as &str for replay_to_lsn(path: &str). Got:\n{rs}"
    );
    assert!(
        !rs.contains("replay_to_lsn(self.path.clone()"),
        "gate-c: must not clone String for replay_to_lsn. Got:\n{rs}"
    );
}

// ── gate-d: append_put params → &Vec at WalRecord::put ───────────────────────────────

#[test]
fn path_bytes_segment_append_put_borrows_key_value_params() {
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
    pub value: Vec<u8>,
}

impl WalRecord {
    pub fn put(lsn: Lsn, key: Vec<u8>, value: Vec<u8>) -> WalRecord {
        WalRecord { lsn: lsn, key: key, value: value }
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
    pub bytes: Vec<u8>,
    pub next_lsn: Lsn,
}

impl WalSegment {
    pub fn append_put(self, key: Vec<u8>, value: Vec<u8>) -> Lsn {
        let lsn = self.next_lsn
        let record = WalRecord::put(lsn, key, value)
        let _ = record.key
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
use crate::wal::lsn::Lsn

pub fn run() {
    let mut seg = WalSegment {
        bytes: Vec::new(),
        next_lsn: Lsn { value: 1 },
    }
    seg.append_put(vec![1u8], vec![2u8])
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("wal/segment.rs").expect("wal/segment.rs");
    let put_call = rs.contains("WalRecord::put(");
    let borrows_key = rs.contains("&key") || rs.contains("& key");
    let forwards_ref_key = rs.contains("key: &Vec") && rs.contains(", key,") || rs.contains(", key)");
    assert!(
        put_call && (borrows_key || forwards_ref_key),
        "gate-d: append_put must borrow key at WalRecord::put call. Got:\n{rs}"
    );
    let borrows_value = rs.contains("&value") || rs.contains("& value");
    let forwards_ref_value = rs.contains("value: &Vec") && (rs.contains(", value)") || rs.contains(", value,"));
    assert!(
        put_call && (borrows_value || forwards_ref_value),
        "gate-d: append_put must borrow value at WalRecord::put call. Got:\n{rs}"
    );
}

// ── gate-e: append_delete param → &Vec at WalRecord::delete ─────────────────────────

#[test]
fn path_bytes_segment_append_delete_borrows_key_param() {
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

impl WalRecord {
    pub fn delete(lsn: Lsn, key: Vec<u8>) -> WalRecord {
        WalRecord { lsn: lsn, key: key }
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
    pub bytes: Vec<u8>,
    pub next_lsn: Lsn,
}

impl WalSegment {
    pub fn append_delete(self, key: Vec<u8>) -> Lsn {
        let lsn = self.next_lsn
        let record = WalRecord::delete(lsn, key)
        let _ = record.key
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
use crate::wal::lsn::Lsn

pub fn run() {
    let mut seg = WalSegment {
        bytes: Vec::new(),
        next_lsn: Lsn { value: 1 },
    }
    seg.append_delete(vec![9u8])
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("wal/segment.rs").expect("wal/segment.rs");
    let delete_call = rs.contains("WalRecord::delete(");
    let borrows_key = rs.contains("&key") || rs.contains("& key");
    let forwards_ref_key = rs.contains("key: &Vec") && (rs.contains(", key)") || rs.contains(", key,"));
    assert!(
        delete_call && (borrows_key || forwards_ref_key),
        "gate-e: append_delete must borrow key at WalRecord::delete call. Got:\n{rs}"
    );
}

// ── gate-f: replay borrows self.bytes field at decode_records ────────────────────────

#[test]
fn path_bytes_segment_replay_borrows_bytes_field() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "wal/record.wj",
        r#"
pub struct WalRecord {
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
use crate::wal::record::WalRecord
use crate::wal::record::decode_records

pub struct WalSegment {
    pub bytes: Vec<u8>,
}

impl WalSegment {
    pub fn replay(self) -> Vec<WalRecord> {
        decode_records(self.bytes)
    }
}
"#,
    );
    test.add_file("wal/mod.wj", "pub mod record\npub mod segment\n");
    test.add_file(
        "main.wj",
        r#"
use crate::wal::segment::WalSegment

pub fn run() {
    let seg = WalSegment { bytes: vec![1u8] }
    let _ = seg.replay()
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("wal/segment.rs").expect("wal/segment.rs");
    assert!(
        rs.contains("decode_records(&self.bytes)")
            || rs.contains("decode_records(& self.bytes)"),
        "gate-f: replay must borrow self.bytes at decode_records(&Vec) call. Got:\n{rs}"
    );
}

// ── gate-g: full wal-crate layout — rustc cargo check (integration gate) ───────────────

#[test]
fn path_bytes_wal_layout_rustc_cargo_check() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "wal_layout.wj",
        r#"
extern fn wal_snapshot_load_ffi(path: string) -> Vec<u8>

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

pub fn decode_records(bytes: Vec<u8>) -> Vec<WalRecord> {
    let _ = bytes.len()
    Vec::new()
}

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

    pub fn from_bytes(bytes: Vec<u8>) -> WalSegment {
        WalSegment { bytes: bytes, next_lsn: Lsn { value: 1 } }
    }

    pub fn append_put(self, key: Vec<u8>, value: Vec<u8>) -> Lsn {
        let lsn = self.next_lsn
        let record = WalRecord::put(lsn, key, value)
        let _ = record.key
        self.next_lsn = lsn.next()
        lsn
    }

    pub fn append_delete(self, key: Vec<u8>) -> Lsn {
        let lsn = self.next_lsn
        let record = WalRecord::delete(lsn, key)
        let _ = record.key
        self.next_lsn = lsn.next()
        lsn
    }

    pub fn replay(self) -> Vec<WalRecord> {
        decode_records(self.bytes)
    }
}

pub struct WalWriter {
    pub path: string,
    pub segment: WalSegment,
}

impl WalWriter {
    pub fn open_existing(path: string) -> WalWriter {
        let snapshot = wal_snapshot_load_ffi(path)
        let segment = if snapshot.len() > 0 {
            WalSegment::from_bytes(snapshot)
        } else {
            WalSegment::with_header()
        }
        WalWriter { path: path, segment: segment }
    }

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
"#,
    );
    test.add_file(
        "main.wj",
        r#"
use crate::wal_layout::WalWriter

pub fn run() {
    let writer = WalWriter::open_existing("wal.log")
    let _ = writer.replay_all_records()
    let _ = writer.replay_through(crate::wal_layout::Lsn { value: 5 })
}
"#,
    );

    // Gate: generated Rust must compile under rustc (mirrors wal-crate/gen).
    test.assert_compiles_without_error();
}

// ── gate-h: optional external dogfood wal crate — cargo check (e2e) ─────────
// Enabled only when WJ_DOGFOOD_CRATES_DIR points at a crates directory.

fn dogfood_crates_root() -> Option<std::path::PathBuf> {
    std::env::var_os("WJ_DOGFOOD_CRATES_DIR").map(std::path::PathBuf::from)
}

fn wj_build_dogfood_crate(name: &str) -> bool {
    use std::fs;
    use std::process::Command;

    let Some(root) = dogfood_crates_root() else {
        return false;
    };
    let crate_root = root.join(name);
    assert!(
        crate_root.is_dir(),
        "dogfood crate {name} not found at {}",
        crate_root.display()
    );
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
    true
}

fn cargo_check_dogfood_gen(name: &str) -> bool {
    use std::process::Command;

    let Some(root) = dogfood_crates_root() else {
        return false;
    };
    let gen = root.join(name).join("gen");
    let output = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&gen)
        .output()
        .unwrap_or_else(|e| panic!("spawn cargo check {name}: {e}"));
    assert!(
        output.status.success(),
        "gate-h: cargo check {name}/gen failed:\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

#[test]
fn path_bytes_dogfood_wal_cargo_check() {
    if dogfood_crates_root().is_none() {
        return;
    }
    assert!(wj_build_dogfood_crate("wal-crate"));
    assert!(cargo_check_dogfood_gen("wal-crate"));
}
