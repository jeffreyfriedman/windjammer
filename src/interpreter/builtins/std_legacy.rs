//! Standard-library functions that older or engine-facing modules declare (`std/test`, `std/testing`,
//! `std/encoding` JSON helpers, `std/async`, `std/compute`, `env::vars`) and the ones the interpreter refuses
//! by name (`std/dialog`, `std/storage`, `std/net`, legacy `std/fs` and `std/process` entry points, `random::random`).
//!
//! A refusal is explicit and total: it stops the run with `unsupported`, which makes `wj run` take the compiled path
//! (ADR-0022), and the message names the interpreter-side alternative. Assertion texts and results follow the runtime
//! functions of the same name (`windjammer_runtime::{test, testing}`).

use super::*;
use crate::interpreter::error::{panic_ctl, unsupported};

macro_rules! alias {
    ($r:ident, $from:literal => $to:literal) => {
        $r.func($from, |it, a| it.call_builtin_fn($to, a));
    };
}

macro_rules! refuse {
    ($r:ident, $name:literal, $why:literal) => {
        $r.func($name, |_, _| Err(unsupported(concat!("`std::", $name, "`: ", $why, "; run it compiled (`wj run --compiled`)"))));
    };
}

fn assertion(ok: bool, text: impl FnOnce() -> R<String>) -> R {
    if ok {
        Ok(Value::Unit)
    } else {
        Err(panic_ctl(text()?))
    }
}

fn eq_pair(it: &mut Interpreter, a: &[Value], name: &str) -> R<(Value, Value, String, String)> {
    let (l, r) = (arg(a, 0, name)?, arg(a, 1, name)?);
    let (ls, rs) = (it.debug_string(&l)?, it.debug_string(&r)?);
    Ok((l, r, ls, rs))
}

pub fn register(r: &mut Registry) {
    // ----- std::test (runtime `test.rs`) -----
    for name in ["test::assert", "test::assert_msg"] {
        r.func(name, |_, a| {
            let ok = matches!(arg(&a, 0, "assert")?, Value::Bool(true));
            assertion(ok, || {
                let msg = if a.len() > 1 { arg_str(&a, 1, "assert")?.to_string() } else { String::new() };
                Ok(format!("Assertion failed: {msg}"))
            })
        });
    }
    r.func("test::assert_eq", |it, a| {
        let (l, rr, ls, rs) = eq_pair(it, &a, "assert_eq")?;
        assertion(l.structural_eq(&rr), || Ok(format!("Assertion failed: {ls} != {rs}")))
    });
    r.func("test::assert_ne", |it, a| {
        let (l, rr, ls, rs) = eq_pair(it, &a, "assert_ne")?;
        assertion(!l.structural_eq(&rr), || Ok(format!("Assertion failed: {ls} == {rs}")))
    });
    // ----- std::testing (runtime `testing.rs`) -----
    r.func("testing::assert_eq", |it, a| {
        let (l, rr, ls, rs) = eq_pair(it, &a, "assert_eq")?;
        let msg = if a.len() > 2 { arg_str(&a, 2, "assert_eq")?.to_string() } else { String::new() };
        assertion(l.structural_eq(&rr), || Ok(format!("Assertion failed: {msg}\n  left: {ls}\n right: {rs}")))
    });
    r.func("testing::assert_ne", |it, a| {
        let (l, rr, ls, _) = eq_pair(it, &a, "assert_ne")?;
        let msg = if a.len() > 2 { arg_str(&a, 2, "assert_ne")?.to_string() } else { String::new() };
        assertion(!l.structural_eq(&rr), || Ok(format!("Assertion failed: {msg}\n  values should not be equal: {ls}")))
    });

    // ----- std::encoding JSON helpers: the `std::json` functions under their encoding names -----
    alias!(r, "encoding::json_encode" => "json::to_string");
    alias!(r, "encoding::json_encode_pretty" => "json::to_string_pretty");
    alias!(r, "encoding::json_decode" => "json::parse_string");

    // ----- std::async / std::compute: run on the interpreter's cooperative scheduler -----
    r.func("async::block_on", |it, a| {
        let f = arg(&a, 0, "block_on")?;
        it.await_value(f)
    });
    r.func("async::spawn", |_, a| arg(&a, 0, "spawn")); // `std/async.wj`: the stub returns its argument
    r.func("compute::num_workers", |_, _| {
        Ok(Value::Int(std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as i128, IntTy::I64))
    });
    r.func("compute::parallel", |it, a| {
        let items = it.to_items(arg(&a, 0, "parallel")?)?;
        let f = arg(&a, 1, "parallel")?;
        let mut out = Vec::with_capacity(items.len());
        for x in items {
            out.push(it.call_value(&f, vec![x])?);
        }
        Ok(Value::vec(out))
    });
    r.func("compute::background", |it, a| {
        let f = arg(&a, 0, "background")?;
        it.call_value(&f, vec![]) // a finished future: `.await` of a plain value is the value
    });
    r.func("compute::join", |it, a| {
        let (fa, fb) = (arg(&a, 0, "join")?, arg(&a, 1, "join")?);
        let x = it.call_value(&fa, vec![])?;
        let y = it.call_value(&fb, vec![])?;
        Ok(Value::tuple(vec![x, y]))
    });
    r.func("compute::map_reduce", |it, a| {
        let items = it.to_items(arg(&a, 0, "map_reduce")?)?;
        let (m, red) = (arg(&a, 1, "map_reduce")?, arg(&a, 2, "map_reduce")?);
        let mut acc = arg(&a, 3, "map_reduce")?;
        for x in items {
            let mapped = it.call_value(&m, vec![x])?;
            acc = it.call_value(&red, vec![acc, mapped])?;
        }
        Ok(acc)
    });

    // ----- std::env::vars (an `env.read` of every variable) -----
    r.func("env::vars", |it, _| {
        // the snapshot the runner passed in; the `env.read(*)` sink row gates the call
        let vars = it.env_snapshot.clone();
        Ok(Value::vec(vars.into_iter().map(|(k, v)| Value::tuple(vec![Value::str(k), Value::str(v)])).collect()))
    });

    // ----- refusals -----
    refuse!(r, "random::random", "`random<T>()` needs the result type; use `random::range`, `random::float` or `random::bool`");
    refuse!(r, "dialog::open_file", "dialogs need a display");
    refuse!(r, "dialog::open_directory", "dialogs need a display");
    refuse!(r, "dialog::save_file", "dialogs need a display");
    refuse!(r, "dialog::show_message", "dialogs need a display");
    refuse!(r, "dialog::show_confirm", "dialogs need a display");
    for_storage(r);
    refuse!(r, "net::get", "the engine HTTP client is not interpreted; use `std::http::get`");
    refuse!(r, "net::get_async", "the engine HTTP client is not interpreted; use `std::http::get`");
    refuse!(r, "net::post", "the engine HTTP client is not interpreted; use `std::http::post`");
    refuse!(r, "net::post_async", "the engine HTTP client is not interpreted; use `std::http::post`");
    refuse!(r, "net::download", "downloads are not interpreted; use `std::http::get` and `std::fs::write`");
    refuse!(r, "net::upload", "uploads are not interpreted; use `std::http::post`");
    refuse!(r, "process::execute", "the engine process API is not interpreted; use `std::process::run` or `run_with_args`");
    refuse!(r, "process::spawn", "the engine process API is not interpreted; use `std::subprocess`");
    refuse!(r, "fs::read_file", "the engine file API is not interpreted; use `std::fs::read_to_string`");
    refuse!(r, "fs::write_file", "the engine file API is not interpreted; use `std::fs::write`");
    refuse!(r, "fs::list_directory", "the engine file API is not interpreted; use `std::fs::read_dir`");
    refuse!(r, "fs::create_directory", "the engine file API is not interpreted; use `std::fs::create_dir_all`");
    refuse!(r, "fs::delete_file", "the engine file API is not interpreted; use `std::fs::remove_file`");
    refuse!(r, "fs::file_exists", "the engine file API is not interpreted; use `std::fs::exists`");
}

/// `std::storage`: a key-value store with backends and TTLs that only the compiled runtime provides.
fn for_storage(r: &mut Registry) {
    refuse!(r, "storage::get", "the storage API is not interpreted");
    refuse!(r, "storage::set", "the storage API is not interpreted");
    refuse!(r, "storage::has", "the storage API is not interpreted");
    refuse!(r, "storage::keys", "the storage API is not interpreted");
    refuse!(r, "storage::remove", "the storage API is not interpreted");
    refuse!(r, "storage::clear", "the storage API is not interpreted");
    refuse!(r, "storage::get_bytes", "the storage API is not interpreted");
    refuse!(r, "storage::set_bytes", "the storage API is not interpreted");
    refuse!(r, "storage::get_json", "the storage API is not interpreted");
    refuse!(r, "storage::set_json", "the storage API is not interpreted");
    refuse!(r, "storage::get_with_ttl", "the storage API is not interpreted");
    refuse!(r, "storage::set_with_ttl", "the storage API is not interpreted");
    refuse!(r, "storage::get_with_backend", "the storage API is not interpreted");
    refuse!(r, "storage::set_with_backend", "the storage API is not interpreted");
    refuse!(r, "storage::keys_with_backend", "the storage API is not interpreted");
    refuse!(r, "storage::remove_with_backend", "the storage API is not interpreted");
    refuse!(r, "storage::clear_with_backend", "the storage API is not interpreted");
}
