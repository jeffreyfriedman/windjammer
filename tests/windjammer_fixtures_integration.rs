//! Runs Windjammer sources in [`tests/windjammer_tests/*.wj`] as Rust `#[test]`s (via codegen + `cargo test`).
//!
//! Converted from embedded snippets in regression / feature tests — behavior is exercised with
//! `windjammer_runtime::test::*` asserts instead of only checking generated Rust shape.

#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
    feature = "skip_fixtures",
)))]

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tempfile::TempDir;
use windjammer::compiler::build_project;
use windjammer::CompilationTarget;

const FIXTURE_TIMEOUT: Duration = Duration::from_secs(180);

/// Shared target directory so fixture tests don't recompile windjammer-runtime
/// from scratch every time. This is the single biggest performance win.
fn shared_fixture_target_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("wj_fixture_verify")
}

/// Serialize fixture cargo invocations to avoid lock-file contention.
static FIXTURE_SUITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn serialized_fixture_suite<T>(f: impl FnOnce() -> T) -> T {
    let lock = FIXTURE_SUITE_LOCK.get_or_init(|| Mutex::new(()));
    let _hold = lock.lock().unwrap_or_else(|p| p.into_inner());
    f()
}

/// Strip duplicate compiler-emitted `use windjammer_runtime::test::*` lines.
fn normalize_generated_integration_body(code: &str) -> String {
    let mut saw_runtime_test_use = false;
    let mut out = String::new();
    for line in code.lines() {
        let t = line.trim();
        if t.starts_with("use windjammer_runtime::test::") {
            if saw_runtime_test_use {
                continue;
            }
            saw_runtime_test_use = true;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn run_windjammer_fixture_wj(fixture_rel: &str) {
    serialized_fixture_suite(|| inner_run_windjammer_fixture_wj(fixture_rel));
}

fn inner_run_windjammer_fixture_wj(fixture_rel: &str) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixed_dir = manifest.join("tests/windjammer_tests");
    let src_path = fixed_dir.join(fixture_rel);
    let source = fs::read_to_string(&src_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", src_path.display()));

    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    let wj_target = root.join(fixture_rel);
    if let Some(parent) = wj_target.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&wj_target, source).unwrap();

    let out_dir = root.join("wj_out");
    build_project(&wj_target, &out_dir, CompilationTarget::Rust, false).unwrap_or_else(|e| {
        panic!(
            "wj transpile failed for fixture {}:\n{e}",
            src_path.display()
        )
    });

    let rs_name = fixture_rel.replace(".wj", ".rs");
    let gen_path = out_dir.join(Path::new(&rs_name).file_name().unwrap());
    let raw = fs::read_to_string(&gen_path).unwrap_or_else(|e| {
        panic!(
            "read generated {} (from {}) failed: {e}",
            gen_path.display(),
            src_path.display(),
        )
    });
    let body = normalize_generated_integration_body(&raw);

    fs::create_dir_all(root.join("tests")).unwrap();
    let integration_rs = root.join("tests/generated_fixture.rs");
    fs::write(&integration_rs, body).unwrap();

    let rt = manifest.join("crates/windjammer-runtime");
    let cargo_toml = format!(
        r#"[package]
name = "wj_fixture_exec"
version = "0.0.0"
edition = "2021"

[dependencies]
windjammer-runtime = {{ path = "{}" }}

[[test]]
name = "generated_fixture"
path = "tests/generated_fixture.rs"
"#,
        test_utils::path_to_toml_string(&rt)
    );
    fs::write(root.join("Cargo.toml"), cargo_toml).unwrap();

    let shared_target = shared_fixture_target_dir();
    let mut child = Command::new("cargo")
        .current_dir(root)
        .env("CARGO_TARGET_DIR", &shared_target)
        .args(["test", "--test", "generated_fixture"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn cargo test: {e}"));

    let deadline = Instant::now() + FIXTURE_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!(
                        "cargo test TIMED OUT after {}s for fixture {}",
                        FIXTURE_TIMEOUT.as_secs(),
                        src_path.display()
                    );
                }
                std::thread::sleep(Duration::from_millis(250));
            }
            Err(e) => panic!("error waiting for cargo test: {e}"),
        }
    }

    let output = child.wait_with_output()
        .unwrap_or_else(|e| panic!("failed to collect output: {e}"));

    if output.status.success() {
        return;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    panic!(
        "cargo test failed for fixture {}\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
        src_path.display()
    );
}

macro_rules! fixture {
    ($fname:ident, $relative:literal) => {
        #[test]
        fn $fname() {
            run_windjammer_fixture_wj($relative);
        }
    };
}

fixture!(feature_basic_math, "feature_basic_math_test.wj");
fixture!(
    feature_increment_local_assignment,
    "feature_assignment_increment_test.wj"
);
fixture!(feature_if_else_sign, "feature_if_else_sign_test.wj");
fixture!(
    feature_string_interpolate,
    "feature_string_interpolate_test.wj"
);
fixture!(feature_pipe_operator, "feature_pipe_operator_test.wj");
fixture!(regression_match_break, "regression_match_break_test.wj");
fixture!(
    regression_match_continue,
    "regression_match_continue_test.wj"
);
fixture!(regression_match_return, "regression_match_return_test.wj");
fixture!(
    regression_count_unique_if_no_else,
    "regression_count_unique_if_no_else_test.wj"
);
fixture!(
    regression_typed_literals_accumulator,
    "regression_typed_literals_accumulator_test.wj"
);
fixture!(
    regression_f32_angle_compute,
    "regression_f32_angle_compute_test.wj"
);
fixture!(
    regression_f32_tile_bounds,
    "regression_f32_tile_bounds_test.wj"
);
fixture!(
    regression_f32_literal_scale_field,
    "regression_f32_literal_scale_field_test.wj"
);
fixture!(
    regression_outer_inner_area,
    "regression_outer_inner_area_test.wj"
);
fixture!(
    regression_engine_cached_area,
    "regression_engine_cached_area_test.wj"
);
fixture!(
    regression_type_alias_objective_field,
    "regression_type_alias_objective_field_test.wj"
);
fixture!(
    regression_type_alias_enum_quest_status,
    "regression_type_alias_enum_quest_status_test.wj"
);
fixture!(
    regression_hashmap_contains_string,
    "regression_hashmap_contains_string_test.wj"
);
fixture!(
    regression_hashmap_get_clone_string,
    "regression_hashmap_get_clone_string_test.wj"
);
fixture!(
    regression_f32_add_after_function_return,
    "regression_f32_add_after_function_return_test.wj"
);
fixture!(
    regression_grid_diagonal_literal_multiply,
    "regression_grid_diagonal_literal_multiply_test.wj"
);
fixture!(
    regression_option_vec_pair_sum,
    "regression_option_vec_pair_sum_test.wj"
);
fixture!(
    regression_nested_stack_item_id_fold,
    "regression_nested_stack_item_id_fold_test.wj"
);
fixture!(
    regression_config_optional_some_assign,
    "regression_config_optional_some_assign_test.wj"
);
fixture!(
    regression_enum_kill_payload_extract,
    "regression_enum_kill_payload_extract_test.wj"
);
fixture!(
    regression_vec_tuple_string_push,
    "regression_vec_tuple_string_push_test.wj"
);
fixture!(
    regression_logger_owned_passthrough,
    "regression_logger_owned_passthrough_test.wj"
);
fixture!(
    regression_format_macro_string_literal,
    "regression_format_macro_string_literal_test.wj"
);
fixture!(stdlib_strings_parse, "stdlib_strings_parse_test.wj");
