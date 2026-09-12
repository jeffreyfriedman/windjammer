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

//! WDB-164: owned store formal consumed via `let mut out = store` must stay owned.
//!
//! Product cold gen (`relational_mvcc_*.rs`):
//!   `fn relational_mvcc_put_version(store: &mut RelationalMvccStore, …) -> RelationalMvccStore`
//!   with body `let mut out = store;` → invalid move from `&mut`, plus call sites
//!   `store = relational_mvcc_put_version(&mut store, …)`.
//!
//! Root cause: bare-pass MutBorrowed demotion when consume-via-let-rebind lives under
//! assignment call sites (`store = put_version(store, …)`), which were invisible to
//! the call-hint walk, and no skip/restore for `let mut out = store`.
//!
//! Note: WDB-163 covers early-return `(state, …)` yielding `&mut State`; this gate is
//! the put_version / writeback store formal demotion sibling.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod store
"#;

const STORE: &str = r#"
pub struct Store {
    pub n: i64,
}

pub fn put_version(store: Store, delta: i64) -> Store {
    let mut out = store
    out.n = out.n + delta
    out
}

pub fn seed() -> i64 {
    let mut store = Store { n: 0 }
    store = put_version(store, 7)
    store = put_version(store, 1)
    store.n
}
"#;

fn wdb164_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("store.wj", STORE);
    test
}

#[test]
fn wdb164_module_file_store_consume_rebind_must_stay_owned() {
    let mut test = wdb164_fixture();
    let map = test
        .compile()
        .expect("WDB-164 multipass compile should succeed (codegen may still be wrong)");
    let store_rs = map.get("store.rs").expect("store.rs");

    eprintln!("WDB-164 store.rs:\n{store_rs}");

    let demoted_formal = store_rs.contains("put_version(store: &mut Store")
        || store_rs.contains("put_version(mut store: &mut Store")
        || store_rs.contains("fn put_version(store: &mut ");
    let mut_ref_call = store_rs.contains("put_version(&mut store")
        || store_rs.contains("put_version(&mut ");

    assert!(
        !demoted_formal && !mut_ref_call,
        "WDB-164 RED: store consume-via-let-mut-rebind demoted to &mut. Got:\n{store_rs}"
    );
    assert!(
        store_rs.contains("put_version(mut store: Store")
            || store_rs.contains("put_version(store: Store"),
        "WDB-164: expected owned Store formal. Got:\n{store_rs}"
    );

    test.cargo_check().expect(
        "WDB-164: owned store consume-rebind must cargo-check without &mut formals/calls.",
    );
}

#[test]
fn wdb164_product_full_relational_module_file_must_not_demote_put_version_store() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push(
        "windjammerdb/crates/wdb-layers/gen/relational_module_file/relational_mvcc.rs",
    );

    if !path.exists() {
        let mut alt = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        alt.pop();
        alt.push("windjammerdb/crates/wdb-layers/gen/relational_module_file");
        if !alt.exists() {
            eprintln!("WDB-164: skip product gate — gen/relational_module_file missing");
            return;
        }
        let mut bad = 0usize;
        if let Ok(entries) = std::fs::read_dir(&alt) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) != Some("rs") {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&p) else {
                    continue;
                };
                if text.contains("fn relational_mvcc_put_version(store: &mut ")
                    || text.contains("relational_mvcc_put_version(&mut store")
                {
                    bad += 1;
                    eprintln!("WDB-164 product hit in {}", p.display());
                }
            }
        }
        assert!(
            bad == 0,
            "WDB-164 RED: product gen still demotes put_version store to &mut ({bad} files)"
        );
        return;
    }

    let text = std::fs::read_to_string(&path).expect("read product mvcc gen");
    let demoted = text.contains("fn relational_mvcc_put_version(store: &mut ")
        || text.contains("relational_mvcc_put_version(&mut store");
    eprintln!(
        "WDB-164 product {} demoted={}",
        path.display(),
        demoted
    );

    assert!(
        !demoted,
        "WDB-164 RED: full relational --module-file still demotes store to &mut in {}.",
        path.display()
    );
}
