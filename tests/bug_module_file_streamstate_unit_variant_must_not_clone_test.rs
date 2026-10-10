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
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum StreamState {
    NotLoaded,
    Loading,
    Resident,
    Evicting,
    Playing,
}

pub struct Entry {
    pub state: StreamState,
}

pub fn mark_loading() -> Entry {
    Entry {
        state: StreamState::Loading,
    }
}

pub fn mark_resident() -> Entry {
    Entry {
        state: StreamState::Resident,
    }
}

pub fn mark_playing() -> Entry {
    Entry {
        state: StreamState::Playing,
    }
}
"#;

#[test]
fn module_file_streamstate_unit_variant_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("StreamState::Loading.clone()")
        || rs.contains("StreamState::Resident.clone()")
        || rs.contains("StreamState::NotLoaded.clone()")
        || rs.contains("StreamState::Evicting.clone()")
        || rs.contains("StreamState::Playing.clone()");
    assert!(
        !bad,
        "RED Copy StreamState unit variant cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
