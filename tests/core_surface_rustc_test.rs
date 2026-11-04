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
//! Option C stage C1 gate: every entry of the typed Rust core surface (`std/rust_core.wji`) agrees with rustc.
//! Each entry becomes one Rust function that calls the method on a receiver of the declared type (generics
//! instantiated: element/value `i64`, key `String`, error `String`, a method generic `i64`) with arguments of
//! the declared parameter types and binds the result at the declared result type. rustc must accept the file.

use windjammer::parser::Type;
use windjammer::typing::core_surface;

/// Rust spelling of a surface type with the generics instantiated. `by_ref` instantiates the element as `&i64`
/// (`Option::copied` / `cloned`, whose receiver is `Option<&T>`).
fn rust(t: &Type, by_ref: bool) -> String {
    match t {
        Type::Int => "i64".into(),
        Type::Int32 => "i32".into(),
        Type::Uint => "u32".into(),
        Type::Float => "f64".into(),
        Type::Bool => "bool".into(),
        Type::String => "String".into(),
        Type::Custom(n) | Type::Generic(n) => match n.as_str() {
            "T" | "V" | "F" if by_ref => "&'static i64".into(),
            "T" | "V" | "F" => "i64".into(),
            "K" | "E" => "String".into(),
            "string" | "String" => "String".into(),
            other => other.to_string(),
        },
        Type::Reference(x) => format!("&{}", rust(x, by_ref)),
        Type::MutableReference(x) => format!("&mut {}", rust(x, by_ref)),
        Type::Option(x) => format!("Option<{}>", rust(x, by_ref)),
        Type::Vec(x) => format!("Vec<{}>", rust(x, by_ref)),
        Type::Result(a, b) => format!("Result<{}, {}>", rust(a, by_ref), rust(b, by_ref)),
        Type::Parameterized(n, xs) => {
            format!("{}<{}>", n, xs.iter().map(|x| rust(x, by_ref)).collect::<Vec<_>>().join(", "))
        }
        other => panic!("surface type not mapped to Rust: {other:?}"),
    }
}

fn receiver(type_name: &str, generics: &[String], by_ref: bool) -> String {
    if generics.is_empty() {
        return type_name.to_string();
    }
    let args: Vec<String> = generics.iter().map(|g| rust(&Type::Generic(g.clone()), by_ref)).collect();
    format!("{}<{}>", type_name, args.join(", "))
}

#[test]
fn every_core_surface_entry_typechecks_in_rustc() {
    let mut src = String::from("#![allow(unused, clippy::all)]\nuse std::collections::*;\nfn arg<A>() -> A { unimplemented!() }\n");
    let entries = core_surface::entries();
    assert!(!entries.is_empty());
    for (i, (ty, method)) in entries.iter().enumerate() {
        let (generics, method_generics, params, ret) = core_surface::declared(ty, method).unwrap();
        // `copied` / `cloned` turn an `Option<&T>` into an `Option<T>`: the receiver holds references.
        let by_ref = ty == "Option" && matches!(method.as_str(), "copied" | "cloned");
        let recv = receiver(ty, &generics, by_ref);
        let args: Vec<String> = params.iter().map(|p| format!("arg::<{}>()", rust(p, false))).collect();
        let turbofish = if method_generics.is_empty() { String::new() } else { "::<i64>".to_string() };
        // The result: the declared type, except a method generic's error type is rustc's (`F::Err`).
        let ret_ty = ret.map(|r| rust(&r, false)).unwrap_or_else(|| "()".into());
        let ret_ty = if method == "parse" { "Result<i64, _>".to_string() } else { ret_ty };
        src.push_str(&format!(
            "fn e{i}(mut r: {recv}) {{ let _x: {ret_ty} = r.{method}{turbofish}({}); }}\n",
            args.join(", ")
        ));
    }
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("surface.rs");
    std::fs::write(&file, &src).unwrap();
    let out = std::process::Command::new("rustc")
        .args(["--crate-type", "lib", "--edition", "2021", "--emit", "metadata", "--out-dir"])
        .arg(dir.path())
        .arg(&file)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "std/rust_core.wji disagrees with rustc:\n{}\n--- generated\n{src}",
        String::from_utf8_lossy(&out.stderr)
    );
}
