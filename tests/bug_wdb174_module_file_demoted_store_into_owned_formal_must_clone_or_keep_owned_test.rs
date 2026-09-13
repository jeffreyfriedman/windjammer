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

//! WDB-174: demoted `&Store` local into owned `Store` formal must `.clone()` (or keep owned).
//!
//! Product residual after tip-out sync (~40× Other←&T), e.g. observability_job_store_port:
//!   caller formal demoted `store: &RelationalMvccStore`
//!   callee `relational_mvcc_put_version(store: RelationalMvccStore, …)` stays owned
//!   call `put_version(store, …)` → E0308 expected Store, found &Store.
//!
//! Inverse of WDB-165 (over-borrow into owned). Signature-driven — no hardcoded names.
//! Prefer tip greens over dogfood.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod mvcc
pub mod jobs
"#;

const MVCC: &str = r#"
pub struct MvccStore {
    pub n: int,
    pub tag: string,
}

/// Owned store consumer (product relational_mvcc_put_version keeps owned).
pub fn put_version(store: MvccStore, row_id: int, snap: int) -> MvccStore {
    MvccStore { n: store.n + row_id + snap, tag: store.tag }
}

pub fn list_visible(store: MvccStore, snap: int, limit: int) -> int {
    store.n + snap + limit
}
"#;

const JOBS: &str = r#"
use crate::mvcc::MvccStore
use crate::mvcc::put_version
use crate::mvcc::list_visible

/// Multi-use read-only probes pull demotion toward `&MvccStore` (product job_store).
pub fn store_probe(store: MvccStore) -> bool {
    store.n >= 0 && store.tag.len() >= 0
}

pub fn put_job(store: MvccStore, row_id: int, snap: int) -> MvccStore {
    let _ok = store_probe(store)
    // Product: put_version(store, …) while demoted — must clone/to_owned into owned formal.
    put_version(store, row_id, snap)
}

pub fn list_jobs(store: MvccStore, snap: int) -> int {
    let _ok = store_probe(store)
    list_visible(store, snap, 1000)
}
"#;

fn wdb174_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("mvcc.wj", MVCC);
    test.add_file("jobs.wj", JOBS);
    test
}

#[test]
fn wdb174_module_file_demoted_store_into_owned_formal_must_clone() {
    let test = wdb174_fixture();
    let map = test
        .compile()
        .expect("WDB-174 multipass compile should succeed");
    let mvcc_rs = map.get("mvcc.rs").expect("mvcc.rs");
    let jobs_rs = map.get("jobs.rs").expect("jobs.rs");

    eprintln!("WDB-174 mvcc.rs:\n{mvcc_rs}\njobs.rs:\n{jobs_rs}");

    let callee_owned = {
        let i = mvcc_rs.find("fn put_version").unwrap_or(0);
        let sl = &mvcc_rs[i..mvcc_rs.len().min(i + 140)];
        (sl.contains("store: MvccStore") || sl.contains("store:MvccStore"))
            && !(sl.contains("store: &MvccStore") || sl.contains("store:&MvccStore"))
    };
    let caller_demoted = {
        let i = jobs_rs.find("fn put_job").unwrap_or(0);
        let sl = &jobs_rs[i..jobs_rs.len().min(i + 120)];
        sl.contains("store: &MvccStore") || sl.contains("store:&MvccStore")
    };
    let call_ok = jobs_rs.contains("put_version(store.clone()")
        || jobs_rs.contains("put_version((*store).clone()")
        || jobs_rs.contains("put_version(store.to_owned()")
        || jobs_rs.contains("put_version((*store).to_owned()");
    let call_bad = jobs_rs.contains("put_version(store,")
        && !jobs_rs.contains("put_version(store.clone()")
        && !jobs_rs.contains("put_version((*store)");

    if callee_owned && caller_demoted && call_bad && !call_ok {
        panic!(
            "WDB-174 RED: demoted &MvccStore passed into owned put_version without clone. \
             Product: relational_mvcc_put_version(store, …) with store: &Store. Got:\n{jobs_rs}\n{mvcc_rs}"
        );
    }

    if callee_owned && caller_demoted {
        assert!(
            call_ok || !call_bad,
            "WDB-174: demoted &Store into owned formal must clone. Got:\n{jobs_rs}"
        );
    }
}

#[test]
fn wdb174_product_job_store_must_clone_demoted_store_into_owned_mvcc() {
    // Product-shaped names (observability_job_store_port → relational_mvcc_put_version).
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod relational
pub mod observability
"#,
    );
    test.add_file(
        "relational/relational_mvcc_port.wj",
        r#"
pub struct RelationalMvccStore {
    pub n: int,
    pub tag: string,
}

pub fn relational_mvcc_put_version(
    store: RelationalMvccStore,
    row_id: int,
    snapshot: int,
    _pad: int,
    _value: int,
) -> RelationalMvccStore {
    RelationalMvccStore { n: store.n + row_id, tag: store.tag }
}

pub fn relational_mvcc_list_visible(store: RelationalMvccStore, snapshot: int, limit: int) -> int {
    store.n + snapshot + limit
}
"#,
    );
    test.add_file(
        "observability/observability_job_store_port.wj",
        r#"
use crate::relational::RelationalMvccStore
use crate::relational::relational_mvcc_put_version
use crate::relational::relational_mvcc_list_visible

pub fn job_store_probe(store: RelationalMvccStore) -> bool {
    store.n >= 0 && store.tag.len() >= 0
}

pub fn job_store_put_job(store: RelationalMvccStore, row_id: int, snapshot: int) -> RelationalMvccStore {
    let _ok = job_store_probe(store)
    relational_mvcc_put_version(store, row_id, snapshot, 0, 0)
}

pub fn job_store_load_jobs(store: RelationalMvccStore, snapshot: int) -> int {
    let _ok = job_store_probe(store)
    relational_mvcc_list_visible(store, snapshot, 1000)
}
"#,
    );
    let map = test
        .compile()
        .expect("WDB-174 product-shaped multipass compile should succeed");
    let jobs_rs = map
        .get("observability/observability_job_store_port.rs")
        .expect("observability_job_store_port.rs");
    let mvcc_rs = map
        .get("relational/relational_mvcc_port.rs")
        .expect("relational_mvcc_port.rs");
    let put_owned = mvcc_rs.contains("store: RelationalMvccStore")
        && !mvcc_rs.contains("store: &RelationalMvccStore");
    let caller_demoted = jobs_rs.contains("store: &RelationalMvccStore");
    let bad = put_owned
        && caller_demoted
        && jobs_rs.contains("relational_mvcc_put_version(store,")
        && !jobs_rs.contains("relational_mvcc_put_version(store.clone()");
    assert!(
        !bad,
        "WDB-174 RED: product job_store demoted &store into owned mvcc_put_version without clone. Got:\n{jobs_rs}\n{mvcc_rs}"
    );
}

/// Tip-out / gen residual gate — multipass fixture above may keep owned formals (false-GREEN).
#[test]
fn wdb174_tip_out_job_store_must_clone_demoted_store_into_owned_mvcc() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/obs_tip_out");
    let rel_tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let jobs = if tip.join("observability_job_store_port.rs").exists() {
        tip.join("observability_job_store_port.rs")
    } else {
        gen.join("observability/observability_job_store_port.rs")
    };
    let mvcc = if rel_tip.join("relational_mvcc_port.rs").exists() {
        rel_tip.join("relational_mvcc_port.rs")
    } else {
        gen.join("relational/relational_mvcc_port.rs")
    };
    if !jobs.exists() || !mvcc.exists() {
        eprintln!("WDB-174 tip-out: skip — jobs/mvcc missing");
        return;
    }
    let jobs_text = std::fs::read_to_string(&jobs).expect("jobs");
    let mvcc_text = std::fs::read_to_string(&mvcc).expect("mvcc");
    let put_owned = {
        let i = mvcc_text
            .find("fn relational_mvcc_put_version")
            .unwrap_or(0);
        let sl = &mvcc_text[i..mvcc_text.len().min(i + 160)];
        sl.contains("store: RelationalMvccStore") && !sl.contains("store: &RelationalMvccStore")
    };
    let caller_demoted = jobs_text.contains("fn job_store_put_job(store: &RelationalMvccStore");
    let bare = jobs_text.contains("relational_mvcc_put_version(store,")
        && !jobs_text.contains("relational_mvcc_put_version(store.clone()");
    eprintln!(
        "WDB-174 tip-out put_owned={} demoted={} bare={} path={}",
        put_owned,
        caller_demoted,
        bare,
        jobs.display()
    );
    assert!(
        !(put_owned && caller_demoted && bare),
        "WDB-174 RED: tip-out/gen job_store passes &store into owned mvcc_put_version.\n{}",
        jobs.display()
    );
}
