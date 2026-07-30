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

//! FAILING REPRO (dogfood): ReactiveApp.mount_target("#main").paint_once()
//! must codegen so hybrid shell can remount Home without stacking RAF loops.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn reactive_app_paint_once_should_codegen_with_mount_target() {
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
    pub fn paint_once(self) {
        println!("paint mount={}", self.mount)
    }
}

fn main() {
    ReactiveApp::new("Home".to_string()).mount_target("#main".to_string()).paint_once()
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("paint_once")
        && (result.contains("\"#main\"") || result.contains("#main"))
        && !result.contains("error[E");
    assert!(
        ok,
        "ReactiveApp.mount_target(\"#main\").paint_once() should codegen cleanly. Got:\n{}",
        result
    );
}
