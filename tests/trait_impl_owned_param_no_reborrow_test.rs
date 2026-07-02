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

//! E0053: trait defines method with owned param, but auto-borrow in impl
//! changes it to &mut, breaking trait signature match.
//!
//! Reproduces: windjammer-game plugin/audio_plugin.rs initialize(&self, ctx: &mut App)
//! vs trait core.rs initialize(&self, app: App)

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn trait_impl_owned_param_not_reborrowed_when_mutated() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "plugin.wj",
        r#"
pub struct App {
    systems: Vec<string>,
}

impl App {
    pub fn new() -> App {
        App { systems: Vec::new() }
    }
    pub fn record_resource(self, name: string) {
        self.systems.push(name)
    }
}

pub trait Plugin {
    fn initialize(self, app: App) -> Result<(), string>
}
"#,
    );
    test.add_file(
        "audio.wj",
        r#"
use plugin::{App, Plugin}

pub struct AudioPlugin {}

impl Plugin for AudioPlugin {
    fn initialize(self, ctx: App) -> Result<(), string> {
        ctx.record_resource("audio_initialized")
        Ok(())
    }
}
"#,
    );
    test.add_file(
        "mod.wj",
        r#"
pub mod plugin
pub mod audio
"#,
    );

    // The trait defines `app: App` (owned). The impl uses different param name `ctx`.
    // Auto-borrow should NOT change it to `&mut App` in the impl.
    let map = test.compile().expect("compile should succeed");
    let audio = map.get("audio.rs").expect("audio.rs should exist in output");
    assert!(
        !audio.contains("&mut App"),
        "Trait impl param should NOT be auto-borrowed to `&mut App`.\nGenerated audio.rs:\n{}",
        audio
    );
    assert!(
        audio.contains("ctx: App") || audio.contains("mut ctx: App"),
        "Trait impl param should remain owned `App`.\nGenerated audio.rs:\n{}",
        audio
    );
    test.assert_compiles_without_error();
}
