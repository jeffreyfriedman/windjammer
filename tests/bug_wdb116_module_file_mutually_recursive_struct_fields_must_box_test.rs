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

//! WDB-116: mutually recursive owned struct fields emit infinite-size Rust types (E0072).
//!
//! WindjammerDB CQ-C1 (after Phase 605 complete-chain close + fresh `wj build src --module-file`):
//! product `wdb-layers` / `wdb-reducer` `--lib` fail with E0072 on complete-chain result structs
//! that embed parent results in a cycle, e.g.:
//!   EqualityResult { readback: ReadbackResult }
//!   ReadbackResult { inequality: InequalityResult }
//!   InequalityResult { equality: EqualityResult }  // cycle
//!
//! Same pattern appears on observability ops stacks and reducer handoffs after the 12-node
//! pipeline was rewired onto itself through Phase 605.
//!
//! Expected (compiler does the hard work): cyclic owned struct fields emit `Box<T>` (or
//! equivalent indirection) so generated Rust has finite size and cargo-checks.
//! Actual: plain `pub field: OtherStruct` → rustc E0072 recursive type has infinite size.
//!
//! Gate: multipass module-file of A↔B embeds must cargo-check when emit is correct (RED until fixed).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod layer_a
pub mod layer_b
"#;

const LAYER_A: &str = r#"
use crate::layer_b::LayerB
use crate::layer_b::layer_b_cap

pub struct LayerA {
    pub child: LayerB
    pub ok: bool
}

pub fn layer_a_cap() -> LayerA {
    let child = layer_b_cap()
    LayerA {
        child: child,
        ok: true,
    }
}
"#;

const LAYER_B: &str = r#"
use crate::layer_a::LayerA
use crate::layer_a::layer_a_cap

pub struct LayerB {
    pub child: LayerA
    pub ok: bool
}

pub fn layer_b_cap() -> LayerB {
    let child = layer_a_cap()
    LayerB {
        child: child,
        ok: true,
    }
}
"#;

fn wdb116_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("layer_a.wj", LAYER_A);
    test.add_file("layer_b.wj", LAYER_B);
    test
}

#[test]
fn wdb116_module_file_mutually_recursive_struct_fields_must_box_or_cargo_check() {
    let mut test = wdb116_fixture();
    let map = test
        .compile()
        .expect("WDB-116 multipass compile should succeed (codegen may still be wrong)");
    let a_rs = map.get("layer_a.rs").expect("layer_a.rs must be generated");
    let b_rs = map.get("layer_b.rs").expect("layer_b.rs must be generated");

    let boxed = (a_rs.contains("Box<") && a_rs.contains("LayerB"))
        || (b_rs.contains("Box<") && b_rs.contains("LayerA"))
        || a_rs.contains("std::boxed::Box")
        || b_rs.contains("std::boxed::Box");

    if !boxed {
        // Document current emit for the compiler agent.
        eprintln!("WDB-116 RED emit layer_a.rs:\n{a_rs}");
        eprintln!("WDB-116 RED emit layer_b.rs:\n{b_rs}");
    }

    test.cargo_check().expect(
        "WDB-116 RED: cyclic owned struct fields must emit Box<T> (or equivalent) so cargo-check passes. Product: wdb-layers/wdb-reducer E0072 after complete-chain Phase 605.",
    );
}
