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

//! DB-410: owned-self wither must move fields, not clone them.
//!
//! Product `PassBuilder::push_binding(self, …) -> PassBuilder`:
//!   `let mut new_bindings = self.bindings.clone();`
//!   `PassBuilder { graph: self.graph.clone(), bindings: new_bindings,
//!     binding_metas: self.binding_metas.clone(), dependencies: self.dependencies.clone(), … }`
//! WJ is `let mut new_bindings = self.bindings` then `PassBuilder { graph: self.graph, … }`.
//! Distinct from DB-378 (indexed `src[i].clone().field` per field) and DB-358 (`self.clone().method()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Graph {
    pub n: i32,
}

pub struct Builder {
    pub graph: Graph,
    pub items: Vec<i32>,
    pub label: string,
}

impl Builder {
    pub fn push(self, x: i32) -> Builder {
        let mut items = self.items
        items.push(x)
        Builder {
            graph: self.graph,
            items: items,
            label: self.label,
        }
    }
}
"#;

/// Product shape: owned-self wither that only reconstructs via struct lit (no
/// intermediate `let x = self.field`) — PassBuilder::shader / dispatch.
const PRODUCT_SHADER_WITHER: &str = r#"
pub struct Graph {
    pub n: i32,
}

pub struct Builder {
    pub graph: Graph,
    pub items: Vec<i32>,
    pub label: string,
}

impl Builder {
    pub fn with_label(self, label: string) -> Builder {
        Builder {
            graph: self.graph,
            items: self.items,
            label: label,
        }
    }
}
"#;

#[test]
fn module_file_owned_self_wither_must_move_fields() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let cloned_fields = rs.contains("self.items.clone()")
        || rs.contains("self.graph.clone()")
        || rs.contains("self.label.clone()");
    assert!(
        !cloned_fields,
        "owned-self wither cloned fields instead of moving:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

#[test]
fn module_file_struct_lit_only_wither_must_move_fields() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", PRODUCT_SHADER_WITHER);
    let map = test.compile().expect("struct-lit wither compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("struct-lit wither lib.rs:\n{rs}");
    assert!(
        !rs.contains("self.graph.clone()") && !rs.contains("self.items.clone()"),
        "struct-lit-only owned-self wither cloned fields:\n{rs}"
    );
    test.cargo_check()
        .expect("struct-lit wither cargo-check");
}
