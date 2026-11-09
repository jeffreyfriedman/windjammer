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
//! Width findings reduced from compiling a large multi-module program (integer and float widths). Each program
//! in `tests/compiler_findings_widths/` is valid Windjammer; the Rust the compiler generates for it must
//! type-check. Tests that still fail are listed in `.config/known-test-failures.txt` (owner: the numeric
//! solver, convergence C4 / option C).

use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn generated_rust_typechecks(name: &str) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/compiler_findings_widths").join(format!("{name}.wj"));
    typechecks_at(&src, name);
}

fn typechecks_at(src: &Path, name: &str) {
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("out");
    let b = Command::new(WJ).args(["build", "--no-cargo", "--no-lint", "-o"]).arg(&out).arg(&src).output().unwrap();
    assert!(b.status.success(), "wj build failed:\n{}", String::from_utf8_lossy(&b.stderr));
    let rs = out.join(format!("{name}.rs"));
    let r = Command::new("rustc")
        .args(["--crate-type", "lib", "--edition", "2021", "--emit", "metadata", "-A", "warnings", "--out-dir"])
        .arg(dir.path())
        .arg(&rs)
        .output()
        .unwrap();
    assert!(
        r.status.success(),
        "generated Rust for {name} does not type-check:\n{}\n--- generated\n{}",
        String::from_utf8_lossy(&r.stderr),
        std::fs::read_to_string(&rs).unwrap_or_default()
    );
}

/// Literals pushed into vectors of several declared widths take each vector's element width.
#[test]
fn push_literals_take_each_declared_vec_width() {
    generated_rust_typechecks("push_literals_take_each_declared_vec_width");
}

/// `for d in -1..1` next to a base read from an `i32` module constant: the loop variable is `i32`.
#[test]
fn negative_range_beside_i32_constant() {
    generated_rust_typechecks("negative_range_beside_i32_constant");
}

/// An element of an untyped integer list passed to an `i32` parameter: the list is `i32`.
#[test]
fn untyped_int_list_element_to_i32_parameter() {
    generated_rust_typechecks("untyped_int_list_element_to_i32_parameter");
}

/// `if`/`else` literals added to an `i32` field read, inside a method returning `Option<i32>` whose tail is `None`.
#[test]
fn literal_branches_beside_i32_field() {
    generated_rust_typechecks("literal_branches_beside_i32_field");
}

/// A literal pushed onto a local `Vec<i32>` takes the vector's element width, not the returned vector's.
#[test]
fn push_literal_takes_local_vec_width() {
    generated_rust_typechecks("push_literal_takes_local_vec_width");
}

/// A float literal in an `f32` enum payload is `f32`.
#[test]
fn float_literal_in_f32_enum_payload() {
    generated_rust_typechecks("float_literal_in_f32_enum_payload");
}

/// A float accumulator returned as `f32` is `f32`, with its literals.
#[test]
fn float_accumulator_returned_as_f32() {
    generated_rust_typechecks("float_accumulator_returned_as_f32");
}

/// A float literal multiplied with a struct that implements `Mul<f32>` is `f32`.
#[test]
fn float_literal_times_f32_operator() {
    generated_rust_typechecks("float_literal_times_f32_operator");
}

/// Untyped float tuples destructured and divided by an `f32`: the literals are `f32`.
#[test]
fn float_tuple_literals_follow_f32_use() {
    generated_rust_typechecks("float_tuple_literals_follow_f32_use");
}

/// `let a = if i < v.len() { v[i] } else { 0 }` over a `Vec<u64>`: the `else` literal is `u64`.
#[test]
fn if_else_literal_takes_u64_element_width() {
    generated_rust_typechecks("if_else_literal_takes_u64_element_width");
}

/// `Shape::Points(vec![1.5, 2.0])` with a `Vec<f32>` payload: the list's literals are `f32`.
#[test]
fn float_list_in_f32_enum_payload() {
    generated_rust_typechecks("float_list_in_f32_enum_payload");
}

/// The project builds with cargo (for programs that call the runtime, such as `std::strings`).
fn generated_crate_builds(name: &str) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/compiler_findings_widths").join(format!("{name}.wj"));
    let dir = TempDir::new().unwrap();
    let out = dir.path().join("out");
    let b = Command::new(WJ).args(["build", "--no-lint", "-o"]).arg(&out).arg(&src).output().unwrap();
    assert!(
        b.status.success(),
        "generated Rust for {name} does not build:\n{}\n{}",
        String::from_utf8_lossy(&b.stdout),
        String::from_utf8_lossy(&b.stderr)
    );
}

/// `strings.substring` called with bounds computed from `int` parameters (its formals are index slots).
#[test]
fn substring_bound_from_int_variables() {
    generated_crate_builds("substring_bound_from_int_variables");
}

/// A counter passed to `strings.substring` and returned in an `(int, string)` tuple.
#[test]
fn counter_in_int_tuple_return() {
    generated_crate_builds("counter_in_int_tuple_return");
}

/// A tuple literal in a `match` arm next to an arm of type `(int, string)`.
#[test]
fn tuple_literal_in_match_arm() {
    generated_rust_typechecks("tuple_literal_in_match_arm");
}

/// A float expression passed to a `float` parameter is `f64` (no `as f32`).
#[test]
fn float_expression_to_float_parameter() {
    // Inline: the exponent literal `1.0e300` trips the span oracle's binary-location check over repository files.
    let dir = TempDir::new().unwrap();
    let src = "pub fn is_huge(x: float) -> bool {\n    x > 1.0e300\n}\n\npub fn check() -> bool {\n    is_huge(1.0e300 * 10.0)\n}\n";
    let file = dir.path().join("float_expression_to_float_parameter.wj");
    std::fs::write(&file, src).unwrap();
    typechecks_at(&file, "float_expression_to_float_parameter");
}

/// The integer surface on literal receivers: `(-7).abs()`, `2.pow(10)` (a `u32` exponent), `7.div_euclid(-2)`.
#[test]
fn numeric_methods_on_literal_receivers() {
    generated_rust_typechecks("numeric_methods_on_literal_receivers");
}

/// `10.min(3)` is one class; `u8::rotate_left` takes a `u32`; `ten.min(three)` needs no cast.
#[test]
fn numeric_method_argument_widths() {
    generated_rust_typechecks("numeric_method_argument_widths");
}

/// `-9223372036854775807 - 1` folds to one `int` literal (spec case SPEC-NUM-001).
#[test]
fn folded_minimum_int_literal() {
    typechecks_at(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/spec/numeric/SPEC-NUM-001.wj"), "SPEC-NUM-001");
}

/// Float and integer methods on literal receivers (spec case SPEC-STD-005).
#[test]
fn float_and_int_methods_on_literals() {
    typechecks_at(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/spec/stdlib/SPEC-STD-005.wj"), "SPEC-STD-005");
}

/// A float literal passed to a Rust crate function whose surface (`rust/<crate>.wji`) declares an `f32`
/// parameter takes that width (`shapes::half(3.0)`).
#[test]
fn float_literal_to_f32_crate_parameter() {
    let tmp = TempDir::new().unwrap();
    let shapes = tmp.path().join("shapes");
    std::fs::create_dir_all(shapes.join("src")).unwrap();
    std::fs::write(shapes.join("Cargo.toml"), "[package]\nname = \"shapes\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
    std::fs::write(shapes.join("src/lib.rs"), "pub fn half(x: f32) -> f32 {\n    x / 2.0\n}\n").unwrap();
    let app = tmp.path().join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    std::fs::create_dir_all(app.join("rust")).unwrap();
    std::fs::write(app.join("wj.toml"), "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nshapes = { path = \"../shapes\" }\n").unwrap();
    std::fs::write(app.join("rust/shapes.wji"), "pub fn half(x: f32) -> f32\n").unwrap();
    std::fs::write(app.join("src/main.wj"), "fn main() {\n    let h = shapes::half(3.0)\n    println(\"${h}\")\n}\n").unwrap();
    let out = app.join("out");
    let b = Command::new(WJ).args(["build", ".", "--no-lint", "-o"]).arg(&out).current_dir(&app).env("WJ_ALLOW_TOOLCHAIN_MISMATCH", "1").output().unwrap();
    assert!(
        b.status.success(),
        "the crate call does not build:\n{}\n{}\n--- generated\n{}",
        String::from_utf8_lossy(&b.stdout),
        String::from_utf8_lossy(&b.stderr),
        std::fs::read_to_string(out.join("main.rs")).unwrap_or_default()
    );
}
