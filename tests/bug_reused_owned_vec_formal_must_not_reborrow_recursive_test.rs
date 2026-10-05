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
    feature = "codegen_tests",
))]

//! P3.647: recursive calls into owned `Vec` formal must `.clone()` (or demote
//! formal to `&Vec`), not pass `&clips` → E0308.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Clip {
    pub id: u32,
}

pub struct Tree {
    pub root: u32,
}

impl Tree {
    pub fn evaluate(self, clips: Vec<Clip>, t: f32) -> u32 {
        self.evaluate_node(self.root, clips, t)
    }

    fn evaluate_node(self, node_id: u32, clips: Vec<Clip>, t: f32) -> u32 {
        if node_id == 0 {
            return clips[0].id
        }
        let a = self.evaluate_node(node_id - 1, clips, t)
        let b = self.evaluate_node(node_id - 1, clips, t)
        a + b
    }
}
"#;

#[test]
fn reused_owned_vec_formal_must_not_reborrow_recursive() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs").clone();
    eprintln!("P3.647 emit:\n{rs}");
    // Gate on `evaluate_node`'s formal only — outer `evaluate(clips: Vec)` must not
    // make `formal_owned` true when the recursive method demoted to `&Vec` (P3.674).
    let node_formal_owned = rs.lines().any(|l| {
        l.contains("fn evaluate_node") && l.contains("clips: Vec<Clip>") && !l.contains("&Vec")
    }) || {
        // Signature may wrap across lines: `fn evaluate_node(...\n    clips: Vec<Clip>`
        let mut saw_fn = false;
        rs.lines().any(|l| {
            if l.contains("fn evaluate_node") {
                saw_fn = true;
            }
            if saw_fn && l.contains("clips: &Vec<Clip>") {
                saw_fn = false;
                return false;
            }
            if saw_fn && l.contains("clips: Vec<Clip>") {
                saw_fn = false;
                return true;
            }
            if saw_fn && l.contains('{') {
                saw_fn = false;
            }
            false
        })
    };
    let node_formal_borrowed = rs.contains("fn evaluate_node")
        && (rs.contains("clips: &Vec<Clip>") || rs.contains("clips: &[Clip]"));
    if node_formal_owned {
        assert!(
            !rs.contains(", &clips,") && !rs.contains(", &clips)"),
            "P3.647 RED: recursive call reborrows into owned Vec:\n{rs}"
        );
        assert!(
            rs.contains("clips.clone()") || rs.matches("evaluate_node(").count() <= 2,
            "P3.647 RED: multi-use owned Vec must clone at recursive sites:\n{rs}"
        );
    } else {
        assert!(
            node_formal_borrowed,
            "P3.647: expect owned Vec or demoted &Vec on evaluate_node:\n{rs}"
        );
    }
    test.cargo_check().expect("cargo-check");
}
