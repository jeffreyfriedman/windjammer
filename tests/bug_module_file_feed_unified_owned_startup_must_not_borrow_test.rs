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

//! Regression: module file feed unified owned startup must not borrow.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod serve
pub mod feed
"#;

const SERVE: &str = r#"
pub struct ServeState {
    pub phase: int,
}

/// Owned startup.
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
    // on_startup(&state, &encode_startup(…)) — must owned/clone
    on_startup(state.clone(), encode_startup("db", "db"))
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("serve.wj", SERVE);
    test.add_file("feed.wj", FEED);
    test
}

#[test]
fn module_file_owned_startup_must_not_receive_ref_args() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let serve_rs = map.get("serve.rs").expect("serve.rs");
    let feed_rs = map.get("feed.rs").expect("feed.rs");

    eprintln!("serve.rs:\n{serve_rs}\nfeed.rs:\n{feed_rs}");

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
            "RED: owned on_startup received &state/&encode. \
             Got:\n{feed_rs}"
        );
    }
    if owned {
        assert!(
            good && !bad,
            "owned startup must get owned/clone args. Got:\n{feed_rs}"
        );
    }
}
