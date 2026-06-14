//! TDD: Parallel per-file int constraint collection must not collide across files.
//!
//! Bug: `collect_program_constraints` did not call `set_current_file`, so every file
//! used the last prepared file's `current_file_id`. Parallel merge then unified
//! unrelated expressions (same line:col in different files) → 221 false errors on
//! engine transpile (exit 1 at Step 4A).

use tempfile::TempDir;
use windjammer::{build_project_ext, lexer, parser, type_inference, CompilationTarget};

fn parse_with_parser(source: &str, file: &str) -> (parser::Parser, parser::Program<'static>) {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new_with_source(tokens, file.to_string(), source.to_string());
    let program = parser.parse().expect("parse fixture");
    (parser, program)
}

#[test]
fn test_int_parallel_merge_same_line_col_different_files_no_conflicts() {
    // Identical line:col in two files — wrong file_id makes expr_id_cache collide.
    let (_parser_a, prog_a) = parse_with_parser(
        "pub fn fa() -> u32 {\n    1\n}\n",
        "/tmp/a.wj",
    );
    let (_parser_b, prog_b) = parse_with_parser(
        "pub fn fb() -> i32 {\n    2\n}\n",
        "/tmp/b.wj",
    );

    let mut global = type_inference::IntInference::new();
    global.prepare_program(&prog_a);
    global.prepare_program(&prog_b);

    let base = global.clone();
    let partials: Vec<type_inference::IntInference> = [(&prog_a, "/tmp/a.wj"), (&prog_b, "/tmp/b.wj")]
        .into_iter()
        .map(|(program, _path)| {
            let mut local = base.clone();
            local.collect_program_constraints(program);
            local
        })
        .collect();

    for partial in partials {
        global.merge_parallel_state(partial);
    }
    global.finish_solve();

    assert!(
        global.errors.is_empty(),
        "parallel int merge must not produce cross-file false conflicts: {:?}",
        global.errors
    );
}

#[test]
fn test_int_library_parallel_collection_same_line_numbers() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    std::fs::write(
        src.join("a.wj"),
        r#"
pub fn fa() -> u32 {
    1
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("b.wj"),
        r#"
pub fn fb() -> i32 {
    2
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("mod.wj"),
        r#"
pub mod a
pub mod b
"#,
    )
    .unwrap();

    build_project_ext(
        &src.join("mod.wj"),
        &build,
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("library build with parallel int inference should succeed");

    let a_code = std::fs::read_to_string(build.join("a.rs")).unwrap();
    let b_code = std::fs::read_to_string(build.join("b.rs")).unwrap();
    assert!(a_code.contains("1_u32"), "a.wj literal should be u32:\n{a_code}");
    assert!(b_code.contains("2_i32"), "b.wj literal should be i32:\n{b_code}");
}
