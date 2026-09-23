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

//! P3.335 / WDB-361: `-> i32` + `let mut i = 0/1` + `while i < vec.len()` must not emit
//! `i: i32 = N_usize` (autotiler / astar_grid product shape).
//! Preferred emit (P3.425): `i: usize` + `return i as i32` (no `(i as usize)`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod autotile
pub mod astar
"#;

const AUTOTILE: &str = r#"
pub struct Entry {
    pub name: string,
}

pub struct AutoTiler {
    pub rules: Vec<Entry>,
}

impl AutoTiler {
    fn find_rule_index(self, name: string) -> i32 {
        let mut i = 0
        while i < self.rules.len() {
            if self.rules[i].name == name {
                return i as i32
            }
            i = i + 1
        }
        -1
    }
}
"#;

const ASTAR: &str = r#"
pub struct Node {
    f_score: f32,
}

fn find_min_f_score(nodes: Vec<Node>) -> i32 {
    if nodes.len() == 0 {
        return -1
    }
    let mut best_idx = 0
    let mut best_f = nodes[0].f_score
    let mut i = 1
    while i < nodes.len() {
        if nodes[i].f_score < best_f {
            best_f = nodes[i].f_score
            best_idx = i
        }
        i = i + 1
    }
    best_idx as i32
}
"#;

fn bad_i32_counter_usize_literal(rs: &str) -> bool {
    rs.contains(": i32 = 0_usize")
        || rs.contains(": i32 = 1_usize")
        || rs.contains(": i32 = 0_usize;")
        || rs.contains(": i32 = 1_usize;")
}

#[test]
fn i32_return_while_len_counter_must_not_emit_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("autotile.wj", AUTOTILE);
    test.add_file("astar.wj", ASTAR);
    let map = test.compile().expect("P3.335 compile");
    let combined = format!(
        "{}\n{}",
        map.get("autotile.rs").expect("autotile.rs"),
        map.get("astar.rs").expect("astar.rs")
    );
    if bad_i32_counter_usize_literal(&combined) {
        eprintln!("P3.335 RED:\n{combined}");
    }
    assert!(
        !bad_i32_counter_usize_literal(&combined),
        "P3.335: i32 return-width counters must not initialize with _usize:\n{combined}"
    );
    // P3.425 / WDB-361: prefer native usize counters (`return i as i32`). Legacy
    // i32 + `(i as usize)` remains acceptable if widths stay consistent.
    let usize_style = combined.contains("let mut i: usize")
        || combined.contains("let mut best_idx: usize")
        || combined.contains(": usize = 0_usize")
        || combined.contains(": usize = 1_usize");
    let i32_cast_style = combined.contains(" as usize") && combined.contains("while");
    assert!(
        usize_style || i32_cast_style,
        "P3.335: expect usize counter or i32+cast vs len():\n{combined}"
    );
    test.cargo_check().expect("P3.335 cargo-check");
}
