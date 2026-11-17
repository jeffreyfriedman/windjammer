//! Programs the compiled build rejects or fails must not pass interpreted (the compiled program is the oracle,
//! ADR-0038 ladder 2). Each case is a reduced repro from ecosystem work; the expected behaviour is the compiled one.

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn run(name: &str, src: &str) -> (i32, String, String) {
    let scratch = Scratch::new(&format!("agree-{name}"));
    let dir = scratch.path().to_path_buf();
    let p = dir.join("main.wj");
    std::fs::write(&p, src).unwrap();
    let o = Command::new(WJ).args(["run", "--interpret", "--no-lint"]).arg(&p).stdin(std::process::Stdio::null()).output().unwrap();
    (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).into(), String::from_utf8_lossy(&o.stderr).into())
}

/// `text.chars().len()`: Rust's `Chars` has no `len` (E0599 in the compiled build), so the interpreted run fails
/// too, naming `.count()`; the same through a size-keeping adapter. ADR-0039 decision H: `.len()` on an iterator is WJ0035
/// ("an iterator is used as a list") with `.count()` as its fix, from the checker or, for a program that skipped it, the
/// interpreter's guard with the same wording.
#[test]
fn len_of_chars_is_refused_like_the_compiled_build() {
    for (name, expr) in [("chars", "\"héllo\".chars().len()"), ("char_indices", "\"héllo\".char_indices().len()"), ("mapped", "\"abc\".chars().map(|c| c).len()")] {
        let (code, out, err) = run(name, &format!("fn main() {{\n    let n = {expr}\n    println(\"${{n}}\")\n}}\n"));
        assert_ne!(code, 0, "{name}: interpreted run passed: {out}");
        assert!(err.contains("an iterator is used as a list") && err.contains("`len`") && err.contains(".count()"), "{name}: {err}");
    }
}

/// What the compiled build accepts still runs: `count()` on `Chars`, `len()` on exact-size iterators.
#[test]
fn exact_size_iterators_keep_len_and_chars_keeps_count() {
    let (code, out, err) = run(
        "ok",
        "fn main() {\n    let v = vec![1, 2, 3]\n    let a = \"héllo\".chars().count()\n    let b = v.iter().len()\n    let c = v.iter().map(|x| x + 1).len()\n    let d = \"héllo\".bytes().len()\n    println(\"${a} ${b} ${c} ${d}\")\n}\n",
    );
    assert_eq!((code, out.as_str()), (0, "5 3 3 6\n"), "{err}");
}

/// An `@test` function that returns `Err` fails, as the compiled harness fails it (a Rust test returning
/// `Result::Err` prints `Error: ..` and fails); one returning `Ok` passes. Found by the ecosystem audit: the
/// interpreted runner dropped the returned value and reported the failing test as ok.
#[test]
fn a_test_returning_err_fails_interpreted_like_the_compiled_harness() {
    let dir = std::env::temp_dir().join(format!("wj-agree-{}-test-err", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("result_test.wj"),
        "fn parse(s: string) -> Result<int, string> {\n    if s == \"1\" {\n        Ok(1)\n    } else {\n        Err(\"not one: ${s}\")\n    }\n}\n\n@test\nfn test_ok() -> Result<(), string> {\n    let n = parse(\"1\")?\n    assert_eq(n, 1)\n    Ok(())\n}\n\n@test\nfn test_err() -> Result<(), string> {\n    let n = parse(\"2\")?\n    assert_eq(n, 2)\n    Ok(())\n}\n",
    )
    .unwrap();
    let o = Command::new(WJ).args(["test", "--interpret"]).current_dir(&dir).stdin(std::process::Stdio::null()).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let (out, err) = (String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert_ne!(o.status.code(), Some(0), "{out}{err}");
    assert!(out.contains("test_ok ... ok"), "{out}{err}");
    assert!(out.contains("test_err ... FAILED"), "{out}{err}");
    assert!(format!("{out}{err}").contains("not one: 2"), "the Err value is reported: {out}{err}");
}

fn project(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("wj-agree-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, text) in files {
        let p = dir.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    dir
}

fn wj_in(dir: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let o = Command::new(WJ).args(args).current_dir(dir).env("WJ_NO_STALE_WARN", "1").stdin(std::process::Stdio::null()).output().unwrap();
    (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).into(), String::from_utf8_lossy(&o.stderr).into())
}

/// A runner in `tools/` and a test in `tests/` import a module from the project root (the manifest's directory), as
/// `wj check` accepts and the reference documents; no folder name is assumed (game logic audit, S1-subdir-import).
#[test]
fn a_file_in_a_subdirectory_imports_a_module_from_the_project_root() {
    let dir = project(
        "subdir-import",
        &[
            ("wj.toml", "[package]\nname = \"subdir_import\"\nversion = \"0.1.0\"\n"),
            ("lib_a.wj", "pub fn double(x: int) -> int {\n    x * 2\n}\n"),
            ("tools/runner.wj", "use lib_a::double\n\nfn main() {\n    println(\"${double(21)}\")\n}\n"),
            ("checks/deep/runner2.wj", "use lib_a::double\n\nfn main() {\n    println(\"${double(4)}\")\n}\n"),
            ("tests/a_test.wj", "use lib_a::double\n\n@test\nfn doubles() {\n    assert_eq(double(2), 4)\n}\n"),
        ],
    );
    let (code, out, err) = wj_in(&dir, &["run", "--interpret", "tools/runner.wj"]);
    assert_eq!((code, out.as_str()), (0, "42\n"), "{err}");
    let (code, out, err) = wj_in(&dir, &["run", "--interpret", "checks/deep/runner2.wj"]);
    assert_eq!((code, out.as_str()), (0, "8\n"), "{err}");
    let (code, out, err) = wj_in(&dir, &["test", "--interpret"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("doubles ... ok"), "{out}{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Two modules each declare `Item`; each decodes JSON into its own (S1 from the game project: the decoder looked the
/// struct up by its bare name and found neither). The drift case `repros/same_named_struct_decode/` holds the compiled
/// result.
#[test]
fn json_decodes_into_the_struct_the_decoding_module_declares() {
    let read = |f: &str| std::fs::read_to_string(format!("{}/tests/drift_corpus/repros/same_named_struct_decode/{f}", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let (main, shop, stock) = (read("main.wj"), read("shop.wj"), read("stock.wj"));
    let dir = project("same-named-decode", &[("main.wj", &main), ("shop.wj", &shop), ("stock.wj", &stock)]);
    let (code, out, err) = wj_in(&dir, &["run", "--interpret", "--no-lint", "main.wj"]);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!((code, out.as_str()), (0, "shop ok 1 a 3\nstock ok 2 c 1\n"), "{err}");
}

/// `f64::from_bits` / `f32::from_bits` decode IEEE bits, as the compiled build does (DbProject I1: the interpreter
/// reported "`u64` has no method `from_bits`"); `to_bits` round-trips.
#[test]
fn float_from_bits_round_trips_like_the_compiled_build() {
    let (code, out, err) = run(
        "from-bits",
        "fn main() {\n    let b: u64 = 4612811918334230528\n    let f = f64::from_bits(b)\n    let g = f32::from_bits(1069547520)\n    println(\"${f} ${f.to_bits() == b} ${g}\")\n}\n",
    );
    assert_eq!((code, out.as_str()), (0, "2.5 true 1.5\n"), "{err}");
}

/// A decode into `other::State` when no module `other` is in reach fails; it never falls back to the `State` the decoding
/// module declares itself (the game host ran `skills.wj` without `state_model.wj`: every `world` input decoded into the
/// skills module's own `State` and was rejected).
#[test]
fn a_decode_into_a_type_of_a_missing_module_never_uses_a_same_named_local_type() {
    let (code, out, err) = run(
        "missing-module-decode",
        "use std::json\n\nstruct State {\n    level: int,\n}\n\nfn main() {\n    let r: Result<other::State, string> = json.parse_string(\"{\\\"level\\\":1}\")\n    match r {\n        Ok(_) => println(\"decoded\"),\n        Err(e) => println(\"err ${e}\"),\n    }\n}\n",
    );
    assert!(code != 0 || (out.starts_with("err ") && out.contains("other")), "decoded into the local State: {out}{err}");
}
