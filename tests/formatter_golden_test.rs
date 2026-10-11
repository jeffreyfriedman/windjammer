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

//! Golden before/after fixtures: `tests/fixtures/formatter/<name>.input.wj` formats to
//! `<name>.expected.wj`, and the expected file is a fixed point.

use std::fs;
use std::path::PathBuf;
use windjammer::formatter::{format_source, FormatOptions};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/formatter")
}

fn check(name: &str) {
    let dir = fixture_dir();
    let input = fs::read_to_string(dir.join(format!("{name}.input.wj"))).unwrap();
    let expected = fs::read_to_string(dir.join(format!("{name}.expected.wj"))).unwrap();
    let opts = FormatOptions::default();
    let got = format_source(&input, &opts).unwrap();
    assert_eq!(got, expected, "fixture `{name}`: formatted output differs");
    assert_eq!(format_source(&expected, &opts).unwrap(), expected, "fixture `{name}` not a fixed point");
}

macro_rules! golden {
    ($($name:ident),* $(,)?) => {
        $( #[test] fn $name() { check(stringify!($name)); } )*
    };
}

golden!(
    spacing,
    braces,
    comments,
    blank_lines,
    match_expr,
    method_chain,
    closure_block,
    decorators,
    strings,
    wrapping,
    trailing_commas,
    generics_impl,
);

#[test]
fn custom_width_and_indent_options_apply() {
    let opts = FormatOptions { max_width: 20, indent: 2 };
    let got = format_source("fn f() {\n    g(aaaa, bbbb, cccc, dddd)\n}\n", &opts).unwrap();
    assert_eq!(got, "fn f() {\n  g(\n    aaaa,\n    bbbb,\n    cccc,\n    dddd,\n  )\n}\n");
}

#[test]
fn fmt_options_read_from_wj_toml() {
    let o = FormatOptions::from_toml_str("[fmt]\nmax_width = 80\nindent = 2\n");
    assert_eq!((o.max_width, o.indent), (80, 2));
    assert_eq!(FormatOptions::from_toml_str("[package]\nname = \"x\"\n"), FormatOptions::default());
}

#[test]
fn untokenizable_input_is_an_error_not_a_panic() {
    assert!(format_source("fn f() { # }", &FormatOptions::default()).is_err());
}
