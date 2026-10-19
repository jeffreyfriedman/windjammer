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

// ---- ADR-0014 intrinsics ------------------------------------------------------------------

fn try_both(src: &str) -> [Result<String, String>; 2] {
    [emit(&GoBackend::new(), src), emit(&JavaScriptBackend::new(), src)]
}

fn ts(src: &str) -> String {
    emit(&JavaScriptBackend::typescript(), src).unwrap_or_else(|e| panic!("ts failed: {e}"))
}

#[test]
fn stderr_intrinsics_route_to_the_stderr_helper() {
    let src = "fn main() {\n    stderr.println(\"e\")\n    stderr.print(\"f\")\n    println(\"o\")\n}\n";
    let g = go(src);
    assert!(g.contains("wjEprint((\"e\") + \"\\n\")") && g.contains("wjEprint((\"f\"))") && g.contains("wjPrint((\"o\") + \"\\n\")"), "{g}");
    for out in [js(src), ts(src)] {
        assert!(out.contains("$eprint((\"e\") + \"\\n\")") && out.contains("$print((\"o\") + \"\\n\")"), "{out}");
    }
}

#[test]
fn lone_string_literal_prints_braces_verbatim() {
    let g = go("fn main() {\n    println(\"{} {x}\")\n}\n");
    assert!(g.contains("\"{} {x}\""), "{g}");
}

#[test]
fn assert_eq_evaluates_operands_once_and_reports_both() {
    let src = "fn f() -> i64 {\n    7\n}\nfn main() {\n    assert_eq(f(), 7, \"ctx\")\n}\n";
    for out in [go(src), js(src)] {
        assert_eq!(out.matches("= f()").count(), 1, "operand evaluated twice:\n{out}");
        assert!(out.contains("left == right") && out.contains("left: ") && out.contains("right: "), "{out}");
    }
}

#[test]
fn todo_and_unreachable_use_the_registry_messages() {
    let src = "fn main() {\n    let n = 1\n    if n == 2 {\n        todo()\n    }\n    if n == 3 {\n        unreachable(\"s ${n}\")\n    }\n}\n";
    for out in [go(src), js(src)] {
        assert!(out.contains("not yet implemented"), "{out}");
        assert!(out.contains("internal error: entered unreachable code: "), "{out}");
    }
}

#[test]
fn debug_returns_its_argument_and_prints_to_stderr() {
    let src = "fn main() {\n    let x: i64 = 4\n    let y = debug(x)\n    println(\"${y}\")\n}\n";
    let g = go(src);
    assert!(g.contains("wjEprint(") && g.contains("x = "), "{g}");
    let j = js(src);
    assert!(j.contains("$eprint(") && j.contains("x = "), "{j}");
}

#[test]
fn user_function_named_like_an_intrinsic_wins() {
    let src = "fn debug(n: i64) -> i64 {\n    n + 1\n}\nfn main() {\n    println(\"${debug(1)}\")\n}\n";
    let j = js(src);
    assert!(j.contains("debug(") && !j.contains("$eprint"), "{j}");
}

#[test]
fn unsupported_format_specs_are_explicit_errors() {
    for spec in ["+", "#x", "E"] {
        let src = format!("fn main() {{\n    let x: i64 = 5\n    println(\"${{x:{spec}}}\")\n}}\n");
        for r in try_both(&src) {
            let e = r.expect_err(&format!("spec `{spec}` must be rejected"));
            assert!(e.contains("unsupported"), "{e}");
        }
    }
    let agg = "fn main() {\n    let v = [1, 2]\n    println(\"${v:#?}\")\n}\n";
    for r in try_both(agg) {
        assert!(r.expect_err("pretty debug of a list").contains("pretty Debug"));
    }
}

#[test]
fn untyped_empty_list_infers_its_element_type_from_push() {
    let g = go("fn main() {\n    let mut v = []\n    v.push(1)\n    println(\"${v.len()}\")\n}\n");
    assert!(g.contains("[]int64{}"), "{g}");
}
