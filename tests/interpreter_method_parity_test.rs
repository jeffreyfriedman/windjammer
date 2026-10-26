#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! Method parity (R5 of the interpreter-readiness scorecard): the interpreter's method registry, not a grep of its
//! source, is held against the shared surface table (`stdlib_surface::known_methods`). A method the interpreter
//! implements that the table lacks would be refused by `wj check` before it runs (`is_ascii_alphanumeric` was), and a
//! method in neither place has no compiled-runtime counterpart (add it to the table only when rustc's std or the
//! runtime has it, otherwise remove the builtin).

use windjammer::interpreter::builtins::Registry;
use windjammer::stdlib_surface::known_methods::is_surface_method;

#[test]
fn every_interpreter_method_is_in_the_surface_table() {
    let missing: Vec<&str> = Registry::standard().method_names().into_iter().filter(|n| !is_surface_method(n) && *n != windjammer::parser::optional_chain::OPT_CHAIN).collect();
    assert!(missing.is_empty(), "interpreter methods missing from the surface table:\n{}", missing.join(", "));
}
