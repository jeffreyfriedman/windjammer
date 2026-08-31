#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! FAILING REPRO — private struct field on Serialize app must not emit spurious `use StructName;`.
//!
//! Ecosystem `wj-webhook`: `struct BusEventBody` + `WebhookApp { bus_queue: Vec<BusEventBody> }`
//! emitted `use BusEventBody;` before the struct definition → E0255 duplicate name.
//! Workaround: move mirror struct to sibling module file (`domain/bus_event.wj`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const WEBHOOK: &str = r#"
struct BusEventBody {
    name: string,
    payload: string,
}

pub struct WebhookApp {
    bus_queue: Vec<BusEventBody>,
}

impl WebhookApp {
    pub fn new() -> WebhookApp {
        WebhookApp { bus_queue: Vec::new() }
    }
}
"#;

#[test]
fn private_struct_field_must_not_emit_spurious_use_import() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod webhook
"#,
    );
    test.add_file("webhook.wj", WEBHOOK);

    let map = test
        .compile()
        .expect("webhook BusEventBody compile should succeed");
    let webhook = map.get("webhook.rs").expect("webhook.rs");

    assert!(
        !webhook.contains("use BusEventBody;"),
        "RED: private struct must not get spurious use import; emitted:\n{webhook}"
    );
    test.cargo_check()
        .expect("WebhookApp with private field struct must cargo-check");
}
