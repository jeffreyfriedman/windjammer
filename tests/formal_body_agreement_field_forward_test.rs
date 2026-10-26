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

//! The body must be generated against the formal that is emitted (ecosystem `wj-grep`, E0382 at
//! `adapters/runner.wj`): the solver says `cmd` is only read through field projections, so the formal is
//! `&Command`. A pub-API restore pass that flips the formal to owned `Command` while the body still
//! assumes a borrow moves `cmd.replace` / `cmd.options` inside the loop. Reduced with `wj reduce`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn formal_and_body_agree_when_options_field_is_forwarded() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "domain.wj",
        r#"
pub struct Options {
    pub width: int,
}

pub struct Command {
    pub paths: Vec<string>,
    pub replace: Option<string>,
    pub options: Options,
}
"#,
    );
    test.add_file(
        "runner.wj",
        r#"
use crate::domain::Command
use crate::domain::Options

fn work(text: string, opts: Options) -> int {
    text.len() + opts.width
}

pub fn run(cmd: Command) -> int {
    let mut total = 0
    for root in &cmd.paths {
        match cmd.replace {
            None => { total = total + work(root.clone(), cmd.options) },
            Some(r) => { total = total + 1 },
        }
    }
    total
}
"#,
    );
    let map = test.compile().expect("multipass compile should succeed");
    let runner = map.get("runner.rs").expect("runner.rs output");
    let owned_formal = runner.contains("fn run(cmd: Command)");
    let body_moves = runner.contains("match cmd.replace {");
    assert!(
        !(owned_formal && body_moves),
        "owned formal with a body that matches `cmd.replace` by value inside a loop (E0382):\n{runner}"
    );
}
