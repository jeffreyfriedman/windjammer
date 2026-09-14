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

//! WDB-211: `fill_bundle_from_six` must not demote fills to `&mut OptOperatorFill`.
//!
//! Product residual (~14×), gen live lag vs tip-out:
//!   gen: `q1: &mut OptOperatorFill` + bare into owned `opt_operator_fill_is_complete`
//!   tip: owned fills + `.clone()` into is_complete. Prefer tip-out → gen sync.
//!
//! Hard-fail tip-out + primary `gen/relational/` only. Stale
//! `gen/relational_module_file/` lag is dogfood sync (not tip RED).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

#[test]
fn wdb211_multipass_fill_bundle_must_keep_owned_fills() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod fills
pub mod bundle
"#,
    );
    test.add_file(
        "fills.wj",
        r#"
pub struct OptOperatorFill {
    pub done: bool,
}

pub fn opt_operator_fill_is_complete(fill: OptOperatorFill) -> bool {
    fill.done
}
"#,
    );
    test.add_file(
        "bundle.wj",
        r#"
use crate::fills::{OptOperatorFill, opt_operator_fill_is_complete}

pub fn fill_bundle_from_six(
    q1: OptOperatorFill,
    q6: OptOperatorFill,
) -> bool {
    let a = opt_operator_fill_is_complete(q1)
    let b = opt_operator_fill_is_complete(q6)
    a && b
}
"#,
    );
    let map = test.compile().expect("WDB-211 multipass");
    let bundle = map.get("bundle.rs").expect("bundle.rs");
    eprintln!("WDB-211 bundle.rs:\n{bundle}");
    assert!(
        !bundle.contains("q1: &mut OptOperatorFill")
            && !bundle.contains("q6: &mut OptOperatorFill"),
        "WDB-211 RED: tip multipass demotes fill_bundle formals to &mut. Got:\n{bundle}"
    );
    assert!(
        bundle.contains("q1: OptOperatorFill") && bundle.contains("q6: OptOperatorFill"),
        "WDB-211: formals must stay owned OptOperatorFill. Got:\n{bundle}"
    );
}

#[test]
fn wdb211_product_live_must_not_demote_fill_bundle_to_mut_ref() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("wave1_opt_live_port.rs"),
        gen.join("relational/wave1_opt_live_port.rs"),
        gen.join("relational_module_file/wave1_opt_live_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("live");
        let bad = text.contains("fill_bundle_from_six(q1: &mut OptOperatorFill")
            || text.contains("q1: &mut OptOperatorFill, q6: &mut OptOperatorFill");
        let path_s = path.to_string_lossy();
        eprintln!("WDB-211 bad={} path={}", bad, path.display());
        // Tip-out + primary gen are tip-truth; module_file lag needs dogfood sync.
        if path_s.contains("rel_tip_out") || path_s.contains("/gen/relational/") {
            assert!(
                !bad,
                "WDB-211 RED: tip/product demotes fill_bundle_from_six to &mut OptOperatorFill. {}",
                path.display()
            );
        } else if bad {
            eprintln!(
                "WDB-211: stale module_file lag (sync tip emit): {}",
                path.display()
            );
        }
    }
    assert!(saw, "WDB-211: live_port missing");
}
