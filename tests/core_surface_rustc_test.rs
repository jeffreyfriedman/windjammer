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
    // `ThreadPanic` is the surface's name for a joined thread's panic payload.
    let mut src = String::from(
        "#![allow(unused, clippy::all)]\nuse std::collections::*;\nuse std::thread::{self, JoinHandle};\nuse std::time::{Duration, Instant};\ntype ThreadPanic = Box<dyn std::any::Any + Send + 'static>;\nfn arg<A>() -> A { unimplemented!() }\n",
    );
    // `Task<T>` is the runtime's (ADR-0004): its module depends on `std` only, so rustc checks the entries against it.
    src.push_str(&format!(
        "#[path = {:?}]\nmod task;\nuse task::Task;\n#[path = {:?}]\nmod sync;\nuse sync::{{Receiver, Sender}};\n",
        concat!(env!("CARGO_MANIFEST_DIR"), "/crates/windjammer-runtime/src/task.rs"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/crates/windjammer-runtime/src/sync.rs")
    ));
    let surface = CoreSurface::parse();
    let entries = surface.entries();
    assert!(!entries.is_empty());
    let mut reading = String::new();
    let mut changing: Vec<(String, String)> = Vec::new();
    for (i, (ty, method)) in entries.iter().enumerate() {
        // An associated function (`Vec::new`) is called on the type, with its generics instantiated.
        if surface.is_static(ty, method) {
            let (generics, _, params, ret) = surface.declared_static(ty, method).unwrap();
            // `@rustc(arg = "i32")`: the type a function-generic argument is checked with (`f64::from` takes no `i64`);
            // `@rustc(ret = "..")`: the result as Rust spells it (`from_str_radix` fails with a `ParseIntError`)
            let rustc_arg = |key: &str| surface.decorators(ty, method).iter().find(|d| d.name == "rustc").and_then(|d| d.args.iter().find(|(k, _)| k == key)).map(|(_, v)| v.clone());
            let args: Vec<String> = params
                .iter()
                .map(|p| match rustc_arg("arg") {
                    Some(a) => format!("arg::<{a}>()"),
                    None => format!("arg::<{}>()", rust(p, false)),
                })
                .collect();
            let ret_ty = rustc_arg("ret").or_else(|| ret.map(|r| rust(&r, false))).unwrap_or_else(|| "()".into());
            // The `Thread` trait declares the `std::thread` module's functions; `Integer`, `F32`, `F64` and `Char` are the primitive types.
            let path = match ty.as_str() {
                "Thread" => "thread".to_string(),
                "Sync" => "sync".to_string(),
                "Integer" => "i64".to_string(),
                "F32" => "f32".to_string(),
                "F64" => "f64".to_string(),
                "Char" => "char".to_string(),
                _ => receiver(ty, &generics, false).replacen('<', "::<", 1),
            };
            // `@constant`: an associated constant (`i64::MAX`), written without a call
            if surface.decorators(ty, method).iter().any(|d| d.name == "constant") {
                src.push_str(&format!("fn e{i}() {{ let _x: {ret_ty} = {path}::{method}; }}\n"));
            } else {
                src.push_str(&format!("fn e{i}() {{ let _x: {ret_ty} = {path}::{method}({}); }}\n", args.join(", ")));
            }
            continue;
        }
        let (generics, method_generics, params, ret) = surface.declared(ty, method).unwrap();
        // `copied` / `cloned` turn an `Option<&T>` into an `Option<T>`: the receiver holds references.
        let by_ref = ty == "Option" && matches!(method.as_str(), "copied" | "cloned");
        // `Integer<T>` and `Float<T>` are the numeric types themselves (checked on `i64` and `f64`).
        // An `Iterator<T>` is checked on a concrete iterator (over references for `cloned` / `copied`).
        let iterator = ty == "Iterator";
        // `@rustc(on = "u64")`: the integer type an entry that Rust declares only on some integer types is checked on
        // (`is_power_of_two` exists on unsigned types, `abs_diff` returns `T` there)
        let checked_on: Option<String> = surface.decorators(ty, method).iter().find(|d| d.name == "rustc").and_then(|d| d.args.iter().find(|(k, _)| k == "on")).map(|(_, v)| v.clone());
        assert!(checked_on.is_none() || ty == "Integer", "`@rustc(on = ..)` on {ty}::{method}: only `Integer` entries are checked on more than one type");
        let recv = match ty.as_str() {
            "Integer" => checked_on.clone().unwrap_or_else(|| "i64".to_string()),
            "Float" => "f64".to_string(),
            "Char" => "char".to_string(),
            // the width traits (`F32::to_bits`) are checked on the type they are named after
            "F32" => "f32".to_string(),
            "F64" => "f64".to_string(),
            "Iterator" if matches!(method.as_str(), "cloned" | "copied") => "std::slice::Iter<'static, i64>".to_string(),
            "Iterator" => "std::vec::IntoIter<i64>".to_string(),
            _ => receiver(ty, &generics, by_ref),
        };
        let args: Vec<String> = params.iter().map(|p| format!("arg::<{}>()", rust(p, false))).collect();
        // Rust's iterator adapters and `Option::map` / `Result::map_err` take their closure type as a generic too: no turbofish there.
        let takes_closure = params.iter().any(|p| matches!(p, Type::FunctionPointer { .. }));
        let turbofish = if method_generics.is_empty() || iterator || takes_closure {
            String::new()
        } else {
            format!("::<{}>", method_generics.iter().map(|g| rust(&Type::Generic(g.clone()), false)).collect::<Vec<_>>().join(", "))
        };
        // The result: the declared type, except a method generic's error type is rustc's (`F::Err`); an iterator
        // result is collected into a `Vec` of its element.
        let (ret_ty, tail) = match ret {
            Some(Type::Parameterized(n, xs)) if n == "Iterator" => (format!("Vec<{}>", rust(&xs[0], false)), ".collect()"),
            // the width traits declare `Vec<u8>`; Rust's `f32::to_le_bytes` is a `[u8; 4]` the lowering turns into a list
            Some(Type::Vec(x)) if ty == "F32" || ty == "F64" => (format!("Vec<{}>", rust(&x, false)), ".to_vec()"),
            r => (r.map(|r| rust(&r, false)).unwrap_or_else(|| "()".into()), ""),
        };
        let ret_ty = if method == "parse" { "Result<i64, _>".to_string() } else { ret_ty };
        // `@rustc(tail = ".collect::<String>()")`: the adapter that turns Rust's result into the declared one (a `char`'s
        // `to_uppercase` is an iterator of characters, lowered to the string they spell)
        let tail = match surface.decorators(ty, method).iter().find(|d| d.name == "rustc").and_then(|d| d.args.iter().find(|(k, _)| k == "tail")) {
            Some((_, t)) => t.as_str(),
            None => tail,
        };
        // `x[a..b]` is parsed as `x.slice(a, b)`: checked as the range index (the result is a slice of the receiver).
        let line = if method == "slice" {
            format!("fn e{i}(mut r: {recv}) {{ let _x = &r[{}..{}]; }}\n", args[0], args[1])
        } else {
            format!("fn e{i}(mut r: {recv}) {{ let _x: {ret_ty} = r.{method}{turbofish}({}){tail}; }}\n", args.join(", "))
        };
        // the entry's own type stands for the checked type: `T` and the element of the generic instantiation are `i64` above
        let numeric = match (ty.as_str(), &checked_on) {
            ("Float", _) => Some("f64".to_string()),
            ("Integer", Some(on)) => Some(on.clone()),
            _ => None,
        };
        src.push_str(&match &numeric {
            Some(n) => line.replace("i64", n),
            None => line.clone(),
        });
        // ADR-0002: the receiver mode. The same call on an immutable binding (`r`, not `mut r`) compiles exactly
        // when the entry is not declared `&mut self`.
        let immutable = line.replacen("(mut r:", "(r:", 1).replacen(&format!("fn e{i}("), &format!("fn m{i}("), 1);
        let immutable = match &numeric {
            Some(n) => immutable.replace("i64", n),
            None => immutable,
        };
        let as_type = match ty.as_str() {
            "Integer" => Type::Int,
            "Float" | "F32" | "F64" => Type::Float,
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
    // the prelude is every line before the first entry function
    let prelude: String = prelude_and_entries.lines().take_while(|l| !l.starts_with("fn e")).map(|l| format!("{l}\n")).collect();
    let prelude_lines = prelude.lines().count();
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
    // each entry is one line after the prelude; rustc names the line of the rejected call
    let rejected: std::collections::BTreeSet<usize> = stderr
        .lines()
        .filter_map(|l| l.trim().strip_prefix("--> "))
        .filter_map(|loc| loc.split(':').nth(1)?.parse::<usize>().ok())
        .collect();
    let accepted: Vec<&str> =
        changing.iter().enumerate().filter(|(i, _)| !rejected.contains(&(i + prelude_lines + 1))).map(|(_, (n, _))| n.as_str()).collect();
    assert!(accepted.is_empty(), "declared `&mut self` in std/rust_core.wji but rustc accepts an immutable receiver: {accepted:?}\n{stderr}");
}
