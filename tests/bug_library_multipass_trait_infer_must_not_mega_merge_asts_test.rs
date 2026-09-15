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

//! Product: tip `wj build --library` of windjammer-game-core (~669 files) was
//! SIGKILL'd (jetsam) immediately after ownership pass 10.
//!
//! Root cause: Step 4B-pre cloned every AST item into one mega-`Program` solely
//! to call `register_traits` + `infer_trait_signatures_from_impls`, doubling peak
//! RSS right when ownership already held a huge registry.
//!
//! Fix: register/infer per-file on each `Program` (same API; no mega merge).
//! This test keeps cross-file trait mut-self inference green under multipass
//! with many sibling modules (stress that we do not need a merged AST).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn library_multipass_infers_cross_file_trait_mut_self_without_merging_all_asts() {
    let mut t = MultiFileTest::new();

    // Many tiny modules — product scale is hundreds of files; we only need
    // enough siblings that a mega-merge would be the wrong design.
    let mut mod_lines = String::from("pub mod ports\npub mod renderer\n");
    for i in 0..48 {
        let name = format!("pad_{:02}", i);
        mod_lines.push_str(&format!("pub mod {}\n", name));
        t.add_file(
            &format!("{}.wj", name),
            &format!("pub fn pad_{}_id() -> i32 {{ {} }}\n", i, i),
        );
    }
    t.add_file("mod.wj", &mod_lines);

    t.add_file(
        "ports.wj",
        r#"
pub trait RenderPort {
    fn initialize()
    fn bump()
}
"#,
    );

    t.add_file(
        "renderer.wj",
        r#"
use crate::ports::RenderPort

pub struct GameRenderer {
    ready: bool,
    frames: i32,
}

impl RenderPort for GameRenderer {
    fn initialize() {
        self.ready = true
    }

    fn bump() {
        self.frames = self.frames + 1
    }
}

pub fn drive(r: GameRenderer) {
    r.initialize()
    r.bump()
}
"#,
    );

    let out = t.compile().expect("multipass compile");
    let rs = out
        .get("renderer.rs")
        .expect("renderer.rs generated");

    assert!(
        rs.contains("&mut self") || rs.contains("&mut GameRenderer"),
        "cross-file RenderPort impl must keep mut self after per-file Step 4B-pre\n{}",
        rs
    );
    assert!(
        !rs.contains("fn initialize(&self)"),
        "initialize must not be &self when body mutates\n{}",
        rs
    );

    t.cargo_check().expect("cargo check multipass fixture");
}
