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
))]

//! Gate (dogfood): ReactiveApp.mount_target("#main")
//! must chain and preserve the selector so WASM remounts do not wipe chrome.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn reactive_app_mount_target_should_preserve_main_selector() {
    // Use r## so "#app" / "#main" do not terminate the raw string early.
    let source = r##"
pub struct ReactiveApp {
    mount: string,
}

impl ReactiveApp {
    pub fn new(_title: string) -> ReactiveApp {
        ReactiveApp { mount: "#app".to_string() }
    }
    pub fn mount_target(self, selector: string) -> ReactiveApp {
        self.mount = selector
        self
    }
    pub fn run(self) {
        println!("mount={}", self.mount)
    }
}

fn main() {
    ReactiveApp::new("Hybrid".to_string()).mount_target("#main".to_string()).run()
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("mount_target")
        && (result.contains("\"#main\"") || result.contains("#main"))
        && !result.contains("error[E");
    assert!(
        ok,
        "ReactiveApp.mount_target(\"#main\") should codegen cleanly. Got:\n{}",
        result
    );
}
