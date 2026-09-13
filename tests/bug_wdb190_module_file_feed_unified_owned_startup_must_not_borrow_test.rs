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

//! WDB-190: gen `feed_unified` must not pass `&state` / `&encode` into owned startup.
//!
//! Product residual (~5× PgWireServeState←& + Vec←&Vec), gen feed_unified:
//!   `pg_wire_serve_on_startup(&state, &pg_wire_encode_startup_user_database(…))`
//! while formals are owned `PgWireServeState` + `Vec<u8>`.
//! Tip-out already emits `on_startup(state.clone(), pg_wire_encode_…(…))` — gen lag.
//! Signature-driven. Prefer tip-out → gen sync.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod serve
pub mod feed
"#;

const SERVE: &str = r#"
pub struct ServeState {
    pub phase: int,
}

/// Owned startup (product pg_wire_serve_on_startup).
pub fn on_startup(state: ServeState, startup: Vec<int>) -> ServeState {
    let _ = startup.len()
    ServeState { phase: state.phase + 1 }
}

pub fn encode_startup(user: string, db: string) -> Vec<int> {
    let _ = user.len() + db.len()
    Vec::new()
}
"#;

const FEED: &str = r#"
use crate::serve::ServeState
use crate::serve::on_startup
use crate::serve::encode_startup

pub fn feed_unified(state: ServeState) -> ServeState {
    // Product gen: on_startup(&state, &encode_startup(…)) — must owned/clone
    on_startup(state.clone(), encode_startup("wdb", "wdb"))
}
"#;

fn wdb190_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("serve.wj", SERVE);
    test.add_file("feed.wj", FEED);
    test
}

#[test]
fn wdb190_module_file_owned_startup_must_not_receive_ref_args() {
    let test = wdb190_fixture();
    let map = test
        .compile()
        .expect("WDB-190 multipass compile should succeed");
    let serve_rs = map.get("serve.rs").expect("serve.rs");
    let feed_rs = map.get("feed.rs").expect("feed.rs");

    eprintln!("WDB-190 serve.rs:\n{serve_rs}\nfeed.rs:\n{feed_rs}");

    let owned = {
        let i = serve_rs.find("fn on_startup").unwrap_or(0);
        let sl = &serve_rs[i..serve_rs.len().min(i + 140)];
        (sl.contains("state: ServeState") || sl.contains("state:ServeState"))
            && !(sl.contains("state: &ServeState") || sl.contains("state:&ServeState"))
    };
    let bad = feed_rs.contains("on_startup(&state")
        || feed_rs.contains("on_startup(&state.clone()")
        || feed_rs.contains(", &encode_startup");
    let good = feed_rs.contains("on_startup(state.clone(), encode_startup")
        || feed_rs.contains("on_startup(state, encode_startup");

    if owned && bad {
        panic!(
            "WDB-190 RED: owned on_startup received &state/&encode. \
             Product gen: pg_wire_serve_on_startup(&state, &encode…). Got:\n{feed_rs}"
        );
    }
    if owned {
        assert!(
            good && !bad,
            "WDB-190: owned startup must get owned/clone args. Got:\n{feed_rs}"
        );
    }
}

#[test]
fn wdb190_product_gen_feed_unified_must_not_borrow_into_owned_startup() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    // Enforce on gen (tip-out already green).
    let feed = gen.join("relational/relational_pg_serve_feed_unified_port.rs");
    let tip_feed = tip.join("relational_pg_serve_feed_unified_port.rs");
    if !feed.exists() {
        eprintln!("WDB-190: skip — gen feed_unified missing");
        return;
    }
    let text = std::fs::read_to_string(&feed).expect("feed");
    let bad = text.contains("pg_wire_serve_on_startup(&state")
        || text.contains("pg_wire_serve_on_startup(&state,")
        || (text.contains("pg_wire_serve_on_startup(")
            && text.contains("&pg_wire_encode_startup_user_database"));
    if tip_feed.exists() {
        let tip_text = std::fs::read_to_string(&tip_feed).unwrap_or_default();
        eprintln!(
            "WDB-190 tip-out clean={}",
            !tip_text.contains("pg_wire_serve_on_startup(&state")
        );
    }
    eprintln!("WDB-190 product gen bad={} path={}", bad, feed.display());
    assert!(
        !bad,
        "WDB-190 RED: product gen feed_unified still borrows into owned on_startup. {}",
        feed.display()
    );
}
