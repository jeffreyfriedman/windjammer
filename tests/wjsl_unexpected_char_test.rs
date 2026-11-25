#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "parser_tests",
))]

/// A character the WJSL lexer cannot tokenize (a `#` line) is a diagnostic with the line,
/// never an empty successful output (WJ0196).
use windjammer::wjsl::transpile_wjsl;

#[test]
fn hash_line_is_an_error_not_an_empty_output() {
    let src = "# not a comment\n@compute @workgroup_size(1, 1, 1)\nfn main() {\n    let x = 1.0;\n}\n";
    let msg = transpile_wjsl(src).expect_err("a # line must not build").to_string();
    assert!(msg.contains("WJ0196"), "{msg}");
    assert!(msg.contains("line 1, column 1"), "{msg}");
    assert!(msg.contains("`//`"), "hint must point at //: {msg}");
}

#[test]
fn hash_inside_a_function_body_names_its_line() {
    let src = "@compute @workgroup_size(1, 1, 1)\nfn main() {\n    let x = 1.0;\n    # trailing\n}\n";
    let msg = transpile_wjsl(src).expect_err("must fail").to_string();
    assert!(msg.contains("line 4, column 5"), "{msg}");
    assert!(msg.contains("# trailing"), "{msg}");
}

#[test]
fn include_line_gets_the_use_hint() {
    let src = "#include \"x.wjsl\"\n";
    let msg = transpile_wjsl(src).expect_err("bare include").to_string();
    assert!(msg.contains("use \"path\""), "{msg}");
}

#[test]
fn slash_comments_and_hash_in_comments_still_build() {
    let src = "// # fine\n@compute @workgroup_size(1, 1, 1)\nfn main() {\n    let x = 1.0; // # also fine\n}\n";
    let wgsl = transpile_wjsl(src).expect("comments build");
    assert!(wgsl.contains("fn main"), "{wgsl}");
}
