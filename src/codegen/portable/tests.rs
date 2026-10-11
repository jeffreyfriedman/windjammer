//! Unit tests for the portable lowering and both emitters (no external toolchains needed).

use crate::codegen::backend::{CodegenBackend, CodegenConfig};
use crate::codegen::go::GoBackend;
use crate::codegen::javascript::JavaScriptBackend;
use std::path::Path;

fn emit(backend: &dyn CodegenBackend, src: &str) -> Result<String, String> {
    let (_parser, program) = crate::compiler::parse_wj_source(Path::new("t.wj"), src).map_err(|e| e.to_string())?;
    backend.generate(&program, &CodegenConfig::default()).map(|o| o.source).map_err(|e| e.to_string())
}

fn js(src: &str) -> String {
    emit(&JavaScriptBackend::new(), src).unwrap_or_else(|e| panic!("js failed: {e}"))
}

fn go(src: &str) -> String {
    emit(&GoBackend::new(), src).unwrap_or_else(|e| panic!("go failed: {e}"))
}

#[test]
fn go_keeps_exact_integer_widths() {
    let out = go("fn main() {\n    let a: i32 = 1\n    let b: i64 = 2\n    let c: u8 = 3\n    println(\"{}\", a)\n    println(\"{}\", b)\n    println(\"{}\", c)\n}\n");
    assert!(out.contains("int32(1)") && out.contains("int64(2)") && out.contains("uint8(3)"), "{out}");
}

#[test]
fn js_represents_wide_ints_as_bigint_and_narrow_as_number() {
    let out = js("fn main() {\n    let a: i32 = 1\n    let b: i64 = 2\n    println(\"{}\", a)\n    println(\"{}\", b)\n}\n");
    assert!(out.contains("let a = 1;") && out.contains("let b = 2n;"), "{out}");
}

#[test]
fn untyped_literals_adopt_usize_from_len_comparison() {
    let out = go("fn main() {\n    let v = vec![1, 2, 3]\n    let mut i = 0\n    while i < v.len() {\n        i += 1\n    }\n}\n");
    assert!(out.contains("var i uint = uint(0)"), "{out}");
}

#[test]
fn struct_assignment_inserts_a_copy_in_js() {
    let out = js("struct P {\n    x: i32,\n}\nfn main() {\n    let a = P { x: 1 }\n    let mut b = a\n    b.x = 5\n    println(\"{}\", a.x)\n}\n");
    assert!(out.contains("$cl(a)") || out.contains("a.clone()"), "{out}");
}

#[test]
fn if_expression_is_flattened_into_statements() {
    let out = js("fn main() {\n    let c = true\n    let x = if c { 1 } else { 2 }\n    println(\"{}\", x)\n}\n");
    assert!(!out.contains("? 1"), "{out}");
    assert!(out.contains("if (c)"), "{out}");
}

#[test]
fn try_operator_becomes_an_early_return() {
    let out = go("fn f(x: i32) -> Result<i32, string> {\n    if x < 0 {\n        return Err(\"neg\")\n    }\n    Ok(x)\n}\nfn g(x: i32) -> Result<i32, string> {\n    let v = f(x)?\n    Ok(v + 1)\n}\nfn main() {\n    println(\"{}\", g(1).unwrap())\n}\n");
    assert!(out.contains("Result[int32, string]"), "{out}");
    assert!(out.contains("return Result[int32, string]{E: "), "{out}");
}

#[test]
fn async_functions_are_rejected_not_miscompiled() {
    let err = emit(&JavaScriptBackend::new(), "async fn f() -> i32 {\n    1\n}\nfn main() {\n    println(\"x\")\n}\n").unwrap_err();
    assert!(err.contains("async"), "{err}");
}

#[test]
fn std_math_calls_use_runtime_shims() {
    let src = "use std::math\nfn main() {\n    println(\"{}\", math::sqrt(2.0))\n}\n";
    assert!(js(src).contains("$std_math_sqrt"));
    assert!(go(src).contains("wjstd_math_sqrt"));
}

#[test]
fn typescript_output_is_annotated() {
    let (_p, program) = crate::compiler::parse_wj_source(Path::new("t.wj"), "fn add(a: i64, b: i64) -> i64 {\n    a + b\n}\nfn main() {\n    println(\"{}\", add(1, 2))\n}\n").unwrap();
    let out = JavaScriptBackend::typescript().generate(&program, &CodegenConfig::default()).unwrap().source;
    assert!(out.contains("a: bigint") && out.contains("): bigint"), "{out}");
}

#[test]
fn removed_js_scaffolding_flags_fail_loudly() {
    let (_p, program) = crate::compiler::parse_wj_source(Path::new("t.wj"), "fn main() {\n    println(\"x\")\n}\n").unwrap();
    let cfg = CodegenConfig { minify: true, ..Default::default() };
    let err = JavaScriptBackend::new().generate(&program, &cfg).unwrap_err().to_string();
    assert!(err.contains("removed"), "{err}");
}
