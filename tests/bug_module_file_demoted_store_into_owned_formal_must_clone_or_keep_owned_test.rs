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

//! Regression: module file demoted store into owned formal must clone or keep owned.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod mvcc
pub mod jobs
"#;

const MVCC: &str = r#"
pub struct MvccStore {
    pub n: int,
    pub tag: string,
}

/// Owned store consumer.
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

/// Multi-use read-only probes pull demotion toward `&MvccStore`.
pub fn store_probe(store: MvccStore) -> bool {
    store.n >= 0 && store.tag.len() >= 0
}

pub fn put_job(store: MvccStore, row_id: int, snap: int) -> MvccStore {
    let _ok = store_probe(store)
    // put_version(store, …) while demoted — must clone/to_owned into owned formal.
    put_version(store, row_id, snap)
}

pub fn list_jobs(store: MvccStore, snap: int) -> int {
    let _ok = store_probe(store)
    list_visible(store, snap, 1000)
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("mvcc.wj", MVCC);
    test.add_file("jobs.wj", JOBS);
    test
}

#[test]
fn module_file_demoted_store_into_owned_formal_must_clone() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let mvcc_rs = map.get("mvcc.rs").expect("mvcc.rs");
    let jobs_rs = map.get("jobs.rs").expect("jobs.rs");

    eprintln!("mvcc.rs:\n{mvcc_rs}\njobs.rs:\n{jobs_rs}");

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
            "RED: demoted &MvccStore passed into owned put_version without clone. \
             Got:\n{jobs_rs}\n{mvcc_rs}"
        );
    }

    if callee_owned && caller_demoted {
        assert!(
            call_ok || !call_bad,
            "demoted &Store into owned formal must clone. Got:\n{jobs_rs}"
        );
    }
}
