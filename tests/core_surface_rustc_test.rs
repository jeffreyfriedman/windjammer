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
use windjammer::typing::core_surface::CoreSurface;

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
            "T" | "V" | "F" | "U" | "A" => "i64".into(),
            "K" | "E" => "String".into(),
            "string" | "String" => "String".into(),
            other => other.to_string(),
        },
        // `&str` and `&String` are one Windjammer type; `&str` is what the String methods return and what
        // a `String`-keyed map accepts.
        Type::Reference(x) if matches!(**x, Type::String) => "&str".into(),
        Type::Reference(x) => format!("&{}", rust(x, by_ref)),
        Type::MutableReference(x) => format!("&mut {}", rust(x, by_ref)),
        Type::Option(x) => format!("Option<{}>", rust(x, by_ref)),
        Type::Vec(x) => format!("Vec<{}>", rust(x, by_ref)),
        Type::Result(a, b) => format!("Result<{}, {}>", rust(a, by_ref), rust(b, by_ref)),
        // An iterator argument: a concrete iterator over the element.
        Type::Parameterized(n, xs) if n == "Iterator" && xs.len() == 1 => format!("std::vec::IntoIter<{}>", rust(&xs[0], by_ref)),
        Type::Parameterized(n, xs) => {
            format!("{}<{}>", n, xs.iter().map(|x| rust(x, by_ref)).collect::<Vec<_>>().join(", "))
        }
        Type::Tuple(xs) => format!("({})", xs.iter().map(|x| rust(x, by_ref)).collect::<Vec<_>>().join(", ")),
        Type::FunctionPointer { params, return_type } => format!(
            "fn({}){}",
            params.iter().map(|x| rust(x, by_ref)).collect::<Vec<_>>().join(", "),
            return_type.as_ref().map(|r| format!(" -> {}", rust(r, by_ref))).unwrap_or_default()
        ),
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
    let surface = CoreSurface::parse();
    let entries = surface.entries();
    assert!(!entries.is_empty());
    let mut reading = String::new();
    let mut changing: Vec<(String, String)> = Vec::new();
    for (i, (ty, method)) in entries.iter().enumerate() {
        // An associated function (`Vec::new`) is called on the type, with its generics instantiated.
        if surface.is_static(ty, method) {
            let (generics, _, params, ret) = surface.declared_static(ty, method).unwrap();
            let args: Vec<String> = params.iter().map(|p| format!("arg::<{}>()", rust(p, false))).collect();
            let ret_ty = ret.map(|r| rust(&r, false)).unwrap_or_else(|| "()".into());
            let path = receiver(ty, &generics, false).replacen('<', "::<", 1);
            src.push_str(&format!("fn e{i}() {{ let _x: {ret_ty} = {path}::{method}({}); }}\n", args.join(", ")));
            continue;
        }
        let (generics, method_generics, params, ret) = surface.declared(ty, method).unwrap();
        // `copied` / `cloned` turn an `Option<&T>` into an `Option<T>`: the receiver holds references.
        let by_ref = ty == "Option" && matches!(method.as_str(), "copied" | "cloned");
        // `Integer<T>` and `Float<T>` are the numeric types themselves (checked on `i64` and `f64`).
        // An `Iterator<T>` is checked on a concrete iterator (over references for `cloned` / `copied`).
        let iterator = ty == "Iterator";
        let recv = match ty.as_str() {
            "Integer" => "i64".to_string(),
            "Float" => "f64".to_string(),
            "Iterator" if matches!(method.as_str(), "cloned" | "copied") => "std::slice::Iter<'static, i64>".to_string(),
            "Iterator" => "std::vec::IntoIter<i64>".to_string(),
            _ => receiver(ty, &generics, by_ref),
        };
        let args: Vec<String> = params.iter().map(|p| format!("arg::<{}>()", rust(p, false))).collect();
        // Rust's iterator adapters take their closure type as a generic too: no turbofish there.
        let turbofish = if method_generics.is_empty() || iterator { String::new() } else { "::<i64>".to_string() };
        // The result: the declared type, except a method generic's error type is rustc's (`F::Err`); an iterator
        // result is collected into a `Vec` of its element.
        let (ret_ty, tail) = match ret {
            Some(Type::Parameterized(n, xs)) if n == "Iterator" => (format!("Vec<{}>", rust(&xs[0], false)), ".collect()"),
            r => (r.map(|r| rust(&r, false)).unwrap_or_else(|| "()".into()), ""),
        };
        let ret_ty = if method == "parse" { "Result<i64, _>".to_string() } else { ret_ty };
        let line = format!("fn e{i}(mut r: {recv}) {{ let _x: {ret_ty} = r.{method}{turbofish}({}){tail}; }}\n", args.join(", "));
        src.push_str(&if ty == "Float" { line.replace("i64", "f64") } else { line.clone() });
        // ADR-0002: the receiver mode. The same call on an immutable binding (`r`, not `mut r`) compiles exactly
        // when the entry is not declared `&mut self`.
        let immutable = line.replacen("(mut r:", "(r:", 1).replacen(&format!("fn e{i}("), &format!("fn m{i}("), 1);
        let immutable = if ty == "Float" { immutable.replace("i64", "f64") } else { immutable };
        let as_type = match ty.as_str() {
            "Integer" => Type::Int,
            "Float" => Type::Float,
            _ => Type::Parameterized(ty.clone(), generics.iter().map(|g| Type::Generic(g.clone())).collect()),
        };
        if surface.changes_receiver(&as_type, method) == Some(true) {
            changing.push((format!("{ty}::{method}"), immutable));
        } else {
            reading.push_str(&immutable);
        }
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
    receiver_modes_agree_with_rustc(&src, &reading, &changing);
}

/// Methods not declared `&mut self` accept an immutable receiver binding; each one declared `&mut self` is rejected
/// on one (E0596). One file per direction; the rejected calls are told apart by the line rustc reports.
fn receiver_modes_agree_with_rustc(prelude_and_entries: &str, reading: &str, changing: &[(String, String)]) {
    assert!(changing.len() >= 30, "too few `&mut self` entries: {}", changing.len());
    let prelude: String = prelude_and_entries.lines().take(3).map(|l| format!("{l}\n")).collect();
    let dir = tempfile::TempDir::new().unwrap();
    let run = |name: &str, body: &str| {
        let file = dir.path().join(name);
        std::fs::write(&file, format!("{prelude}{body}")).unwrap();
        std::process::Command::new("rustc")
            .args(["--crate-type", "lib", "--edition", "2021", "--emit", "metadata", "--out-dir"])
            .arg(dir.path())
            .arg(&file)
            .output()
            .unwrap()
    };
    let out = run("reading.rs", reading);
    assert!(
        out.status.success(),
        "a method not declared `&mut self` in std/rust_core.wji changes its receiver:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: String = changing.iter().map(|(_, l)| l.as_str()).collect();
    let out = run("changing.rs", &body);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // each entry is one line after the 3-line prelude; rustc names the line of the rejected call
    let rejected: std::collections::BTreeSet<usize> = stderr
        .lines()
        .filter_map(|l| l.trim().strip_prefix("--> "))
        .filter_map(|loc| loc.split(':').nth(1)?.parse::<usize>().ok())
        .collect();
    let accepted: Vec<&str> =
        changing.iter().enumerate().filter(|(i, _)| !rejected.contains(&(i + 4))).map(|(_, (n, _))| n.as_str()).collect();
    assert!(accepted.is_empty(), "declared `&mut self` in std/rust_core.wji but rustc accepts an immutable receiver: {accepted:?}\n{stderr}");
}
