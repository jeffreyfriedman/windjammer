// consumes: docs/book/**
//! The Windjammer Book (docs/book): the structure/link/ratchet checks, and every sample in every page run
//! through the interpreter and the compiled path by `scripts/book/book_samples.wj`. A language change that
//! breaks a sample fails the compiler's own test run. The second half plants each class of breakage in a
//! scratch book and requires the gate to notice it.

#![allow(clippy::duplicate_mod)]

#[path = "../tools/docs_truth/mod.rs"]
mod docs_truth;
#[path = "../src/bin/wj_cli_args.rs"]
mod wj_cli_args;

use clap::CommandFactory;
use docs_truth::{book, config, failures, Ctx, Finding};
use std::path::{Path, PathBuf};

fn real() -> Ctx {
    Ctx { root: PathBuf::from(env!("CARGO_MANIFEST_DIR")), cli: wj_cli_args::Cli::command() }
}

/// Compiled runs build real Cargo crates. They get a target directory and a `wj` cache of their own (never the shared
/// one that other tests in the same process build into, which `wj` also prunes), and run one at a time, so no other
/// build can replace a binary between its build and its run.
static COMPILED: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn compiled_lock() -> std::sync::MutexGuard<'static, ()> {
    COMPILED.lock().unwrap_or_else(|e| e.into_inner())
}

fn isolate(cmd: &mut std::process::Command) -> &mut std::process::Command {
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("book-wj");
    cmd.env("CARGO_TARGET_DIR", base.join("target")).env("WJ_CARGO_TARGET_ROOT", base.join("cache"))
}

fn compiled_cmd(wj: &str) -> std::process::Command {
    let mut c = std::process::Command::new(wj);
    isolate(&mut c);
    c.args(["run", "--compiled"]);
    c
}

/// Everything the runner printed, plus how it ended; a runner that died before its summary says so.
fn report(out: &std::process::Output, what: &str) -> String {
    use std::os::unix::process::ExitStatusExt;
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let log = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("book-{what}.log"));
    let _ = std::fs::write(&log, &text);
    eprintln!("{text}");
    let hint = if text.contains("not what the compiler generates") { "\nThe compiler's output changed. Refresh every `rust,generated` fence with `wj-docs book-regen` (cargo run --release --bin wj-docs -- book-regen --wj <path to wj>) and review the diff." } else { "" };
    if text.contains("book samples:") {
        format!("{text}{hint}\n(full output: {})", log.display())
    } else {
        format!("THE RUNNER DID NOT FINISH (status {:?}, signal {:?}); output so far, full copy in {}:\n{text}", out.status.code(), out.status.signal(), log.display())
    }
}

fn show(fs: &[Finding]) -> String {
    fs.iter().map(|f| f.render()).collect::<Vec<_>>().join("\n")
}

fn check(ctx: &Ctx) -> Vec<Finding> {
    book::check_all(ctx, &config::load(ctx))
}

/// Run the sample runner over `ctx`'s book; (success, output).
fn run_samples(ctx: &Ctx, compiled: bool) -> (bool, String) {
    run_samples_with(ctx, compiled, &[])
}

fn run_samples_with(ctx: &Ctx, compiled: bool, env: &[(&str, &str)]) -> (bool, String) {
    let _g = if compiled { Some(compiled_lock()) } else { None };
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("samples");
    let n = book::extract(ctx, &dir).unwrap();
    assert!(n > 0, "the book has no runnable samples");
    let scratch = tmp.path().join("scratch");
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_wj"));
    cmd.args(["run", "--interpret"]).arg(real().root.join("scripts/book/book_samples.wj")).arg("--").arg(&dir).arg(env!("CARGO_BIN_EXE_wj")).arg(&scratch);
    if compiled {
        cmd.arg("--compiled");
        isolate(&mut cmd);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.current_dir(&real().root).output().expect("run scripts/book/book_samples.wj");
    (out.status.success(), report(&out, if compiled { "compiled" } else { "interpreted" }))
}

/// Like `run_samples` (interpreted) but tolerates a book with no `wj` programs, only projects.
fn run_samples_projects(ctx: &Ctx) -> (bool, String) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("samples");
    book::extract(ctx, &dir).unwrap();
    let scratch = tmp.path().join("scratch");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["run", "--interpret"])
        .arg(real().root.join("scripts/book/book_samples.wj"))
        .arg("--")
        .arg(&dir)
        .arg(env!("CARGO_BIN_EXE_wj"))
        .arg(&scratch)
        .current_dir(&real().root)
        .output()
        .expect("run scripts/book/book_samples.wj");
    (out.status.success(), report(&out, "projects"))
}

// ---------------------------------------------------------------------------------------------
// The real book
// ---------------------------------------------------------------------------------------------

#[test]
fn book_structure_links_and_ratchets_hold() {
    let fs = check(&real());
    assert_eq!(failures(&fs), 0, "\n{}", show(&fs));
}

#[test]
fn book_samples_run_interpreted() {
    let (ok, out) = run_samples(&real(), false);
    assert!(ok, "\n{out}");
}

#[test]
fn book_samples_run_compiled_and_match_the_interpreter() {
    let (ok, out) = run_samples(&real(), true);
    assert!(ok, "\n{out}");
}

// ---------------------------------------------------------------------------------------------
// Mutation self-tests on a scratch book
// ---------------------------------------------------------------------------------------------

struct Scratch {
    dir: tempfile::TempDir,
}

const GOOD: &str = "# Page\n\n```wj\n// expect: hi\nfn main() {\n    println(\"hi\")\n}\n```\n";

impl Scratch {
    fn new() -> Scratch {
        let s = Scratch { dir: tempfile::tempdir().unwrap() };
        s.write("docs/docs-truth.toml", "[book]\nmax_ignored = 0\nmax_drafts = 0\nmax_uncovered_promises = 0\n");
        s.write("docs/promises.toml", "[[promise]]\nid = \"wj frob\"\n");
        s.write("docs/book/src/SUMMARY.md", "# Summary\n\n- [Page](page.md)\n");
        s.write("docs/book/src/page.md", &format!("{GOOD}\nRun wj frob.\n"));
        s.write("docs/known-gaps.md", "# Gaps\n");
        s
    }
    fn ctx(&self) -> Ctx {
        Ctx { root: self.dir.path().to_path_buf(), cli: wj_cli_args::Cli::command() }
    }
    fn write(&self, rel: &str, text: &str) {
        let p = self.dir.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    fn page(&self, text: &str) {
        self.write("docs/book/src/page.md", &format!("{text}\nRun wj frob.\n"));
    }
    fn findings(&self) -> Vec<Finding> {
        check(&self.ctx())
    }
    fn fails(&self, needle: &str) -> bool {
        self.findings().iter().any(|f| !f.warn && f.msg.contains(needle))
    }
}

#[test]
fn a_clean_scratch_book_passes() {
    let s = Scratch::new();
    assert_eq!(failures(&s.findings()), 0, "{}", show(&s.findings()));
}

#[test]
fn unlisted_and_missing_pages_fail() {
    let s = Scratch::new();
    s.write("docs/book/src/orphan.md", "# Orphan\n");
    assert!(s.fails("not listed in SUMMARY"));
    let s = Scratch::new();
    s.write("docs/book/src/SUMMARY.md", "# Summary\n\n- [Page](page.md)\n- [Gone](gone.md)\n");
    assert!(s.fails("does not exist"));
}

#[test]
fn broken_links_fail() {
    let s = Scratch::new();
    s.page(&format!("{GOOD}\n[x](nowhere.md)\n"));
    assert!(s.fails("does not exist"));
    s.page(&format!("{GOOD}\n[x](#no-such-heading)\n"));
    assert!(s.fails("no heading"));
    s.page(&format!("{GOOD}\n[x]({}docs/missing.md)\n", book::REPO_URL));
    assert!(s.fails("does not exist"));
    s.page(&format!("{GOOD}\n[x](../../known-gaps.md)\n"));
    assert!(s.fails("leaves the book"));
    s.page(&format!("{GOOD}\n[x]({}docs/known-gaps.md) and [y](#page)\n", book::REPO_URL));
    assert_eq!(failures(&s.findings()), 0, "{}", show(&s.findings()));
}

#[test]
fn a_fence_without_main_fails() {
    let s = Scratch::new();
    s.page("```wj\nprintln(\"x\")\n```\n");
    assert!(s.fails("no `fn main`"));
    // hidden lines complete the program
    s.page("```wj\n~fn main() {\nprintln(\"x\")\n~}\n```\n");
    assert_eq!(failures(&s.findings()), 0, "{}", show(&s.findings()));
}

#[test]
fn ignored_fences_are_a_shrink_only_ratchet() {
    let s = Scratch::new();
    s.page("<!-- ignore: needs a server -->\n```wj,ignore\nfn main() {\n}\n```\n");
    assert!(s.fails("exceeds the ratchet 0"), "{}", show(&s.findings()));
    s.write("docs/docs-truth.toml", "[book]\nmax_ignored = 1\n");
    assert_eq!(failures(&s.findings()), 0, "{}", show(&s.findings()));
    // fixing the example without lowering the ratchet is also a failure: the file must state the truth
    s.page(GOOD);
    assert!(s.fails("below the ratchet 1"));
}

#[test]
fn interpreter_only_fences_need_a_reason_and_are_a_ratchet() {
    let s = Scratch::new();
    s.page("```wj,interpreter_only\nfn main() {\n}\n```\n");
    assert!(s.fails("need a reason"), "{}", show(&s.findings()));
    s.page("<!-- interpreter-only: spawn has no compiled form yet -->\n```wj,interpreter_only\nfn main() {\n}\n```\n");
    assert!(s.fails("interpreter-only fences: 1 exceeds"), "{}", show(&s.findings()));
    s.write("docs/docs-truth.toml", "[book]\nmax_interpreter_only = 1\n");
    assert_eq!(failures(&s.findings()), 0, "{}", show(&s.findings()));
    // the marker file keeps it out of the compiled pass
    let tmp = tempfile::tempdir().unwrap();
    book::extract(&s.ctx(), tmp.path()).unwrap();
    assert!(tmp.path().join("page_L2.interp").exists());
}

#[test]
fn draft_chapters_are_a_shrink_only_ratchet() {
    let s = Scratch::new();
    s.write("docs/book/src/SUMMARY.md", "# Summary\n\n- [Page](page.md)\n- [Later]()\n");
    assert!(s.fails("draft chapters: 1 exceeds"));
}

#[test]
fn uncovered_promises_are_a_shrink_only_ratchet() {
    let s = Scratch::new();
    s.write("docs/promises.toml", "[[promise]]\nid = \"wj frob\"\n[[promise]]\nid = \"code W9999\"\n[[promise]]\nid = \"feature fly\"\n");
    assert!(s.fails("uncovered promises: 2 exceeds"), "{}", show(&s.findings()));
    s.page(&format!("{GOOD}\nW9999 <!-- promise: feature fly -->\n"));
    assert_eq!(failures(&s.findings()), 0, "{}", show(&s.findings()));
}

#[test]
fn a_sample_with_wrong_output_fails_the_runner() {
    let s = Scratch::new();
    let ctx = s.ctx();
    let (ok, out) = run_samples(&ctx, false);
    assert!(ok, "{out}");
    s.page("```wj\n// expect: bye\nfn main() {\n    println(\"hi\")\n}\n```\n");
    let (ok, out) = run_samples(&ctx, false);
    assert!(!ok && out.contains("page.md:1") && out.contains("output differs"), "{out}");
    s.page("```wj\nfn main() {\n    let x: int = \"no\"\n    println(\"${x}\")\n}\n```\n");
    let (ok, out) = run_samples(&ctx, false);
    assert!(!ok, "{out}");
}

#[test]
fn extraction_keeps_hidden_lines_and_expectations() {
    let s = Scratch::new();
    s.page("```wj\n// expect: 3\n~fn main() {\nprintln(\"3\")\n~}\n```\n");
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(book::extract(&s.ctx(), tmp.path()).unwrap(), 1);
    let prog = std::fs::read_to_string(tmp.path().join("page_L1.wj")).unwrap();
    assert!(prog.contains("fn main() {") && !prog.contains('~'), "{prog}");
    assert_eq!(std::fs::read_to_string(tmp.path().join("page_L1.expect")).unwrap(), "3\n");
    assert!(Path::new(tmp.path()).join("INDEX.tsv").exists());
}

// ---------------------------------------------------------------------------------------------
// Findings from writing the book (docs/known-gaps.md, tests/book_repros/): each test passes while the
// bug is present and fails once it is fixed, which is the cue to delete the repro and the known-gaps row.
// ---------------------------------------------------------------------------------------------

#[test]
fn known_gap_generic_copy_out_of_list_fails_compiled() {
    let _g = compiled_lock();
    let repro = real().root.join("tests/book_repros/generic_largest_needs_clone.wj");
    let wj = env!("CARGO_BIN_EXE_wj");
    let interp = std::process::Command::new(wj).args(["run", "--interpret"]).arg(&repro).output().unwrap();
    assert!(interp.status.success() && String::from_utf8_lossy(&interp.stdout).trim() == "9", "the interpreter runs the repro");
    let compiled = compiled_cmd(wj).arg(&repro).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&compiled.stdout), String::from_utf8_lossy(&compiled.stderr));
    assert!(!compiled.status.success() && text.contains("WJ0015"), "the compiled gap is fixed: delete tests/book_repros/generic_largest_needs_clone.wj, this test and its docs/known-gaps.md row\n{text}");
}

#[test]
fn known_gap_idioms_fail_compiled() {
    let _g = compiled_lock();
    let wj = env!("CARGO_BIN_EXE_wj");
    for (file, code) in [("iter_find_closure.wj", "WJ0003"), ("join_unwrap_or_literal.wj", "WJ0003")] {
        let repro = real().root.join("tests/book_repros").join(file);
        let interp = std::process::Command::new(wj).args(["run", "--interpret"]).arg(&repro).output().unwrap();
        assert!(interp.status.success(), "{file}: the interpreter runs the repro");
        let compiled = compiled_cmd(wj).arg(&repro).output().unwrap();
        let text = format!("{}{}", String::from_utf8_lossy(&compiled.stdout), String::from_utf8_lossy(&compiled.stderr));
        assert!(!compiled.status.success() && text.contains(code), "{file}: the compiled gap is fixed: delete the repro, its entry here and its docs/known-gaps.md text\n{text}");
    }
}

#[test]
fn known_gap_module_struct_without_import_fails_compiled() {
    let _g = compiled_lock();
    let src = real().root.join("tests/book_repros/module_struct");
    let tmp = tempfile::tempdir().unwrap();
    let dst = tmp.path().join("p");
    std::fs::create_dir_all(dst.join("src")).unwrap();
    for f in ["wj.toml", "src/main.wj", "src/shapes.wj"] {
        std::fs::copy(src.join(f), dst.join(f)).unwrap();
    }
    let wj = env!("CARGO_BIN_EXE_wj");
    let interp = std::process::Command::new(wj).args(["run", "--interpret"]).current_dir(&dst).output().unwrap();
    assert!(interp.status.success() && String::from_utf8_lossy(&interp.stdout).contains("b 2"), "the interpreter runs the repro");
    let compiled = compiled_cmd(wj).current_dir(&dst).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&compiled.stdout), String::from_utf8_lossy(&compiled.stderr));
    assert!(!compiled.status.success() && text.contains("WJ0003"), "the module-struct gap is fixed: delete tests/book_repros/module_struct, this test and its docs/known-gaps.md row\n{text}");
}

#[test]
fn extraction_writes_multi_file_projects() {
    let s = Scratch::new();
    s.page("```sh,verify\n$ mkdir -p p/src\n$ cat > p/wj.toml <<'EOF'\n[package]\nname = \"p\"\nEOF\n$ cat > p/src/main.wj <<'EOF'\nfn main() {\n    println(\"hi\")\n}\nEOF\n$ cd p && wj run\nhi\n```\n");
    let tmp = tempfile::tempdir().unwrap();
    book::extract(&s.ctx(), tmp.path()).unwrap();
    let base = tmp.path().join("projects/page_L1");
    assert!(base.join("p/wj.toml").exists() && base.join("p/src/main.wj").exists());
    assert_eq!(std::fs::read_to_string(base.join(".root")).unwrap(), "p");
    assert_eq!(std::fs::read_to_string(base.join(".expect")).unwrap(), "hi\n");
}

#[test]
fn a_broken_project_fails_the_runner_interpreted() {
    let s = Scratch::new();
    s.page("```sh,verify\n$ cat > p/wj.toml <<'EOF'\n[package]\nname = \"p\"\nEOF\n$ cat > p/src/main.wj <<'EOF'\nfn main() {\n    println(\"hi\")\n}\nEOF\n$ cd p && wj run\nbye\n```\n");
    let (ok, out) = run_samples_projects(&s.ctx());
    assert!(!ok && out.contains("project output differs"), "{out}");
}

#[test]
fn compiled_samples_do_not_see_each_other_functions() {
    // Two samples define `load` with different signatures; each compiles on its own, so neither can break the other.
    let s = Scratch::new();
    s.page("```wj\n// expect: Err(\"negative\")\nfn load(n: int) -> Result<int, string> {\n    if n < 0 {\n        return Err(\"negative\")\n    }\n    Ok(n)\n}\n\nfn main() {\n    println(\"${load(-1):?}\")\n}\n```\n\n```wj\n// expect: Ok(5)\nfn load(text: string) -> Result<int, string> {\n    text.parse::<int>().map_err(|_| \"bad: ${text}\")\n}\n\nfn main() {\n    println(\"${load(\\\"5\\\"):?}\")\n}\n```\n");
    let (ok, out) = run_samples(&s.ctx(), true);
    assert!(ok, "{out}");
}

#[test]
fn the_compiled_gate_fails_when_compiled_output_differs() {
    // Mutation check for the isolated compiled pass, self-contained: the runner's own test hook makes the compiled
    // output of one sample differ, and the gate must fail naming that sample. No compiler behaviour is involved.
    let s = Scratch::new();
    s.page("```wj\n// expect: hi\nfn main() {\n    println(\"hi\")\n}\n```\n");
    let (ok, out) = run_samples_with(&s.ctx(), true, &[("BOOK_PLANT_COMPILED_DIFF", "page_L1")]);
    assert!(!ok && out.contains("[page_L1]") && out.contains("compiled output differs"), "{out}");
    // and without the hook the same book passes
    let (ok, out) = run_samples(&s.ctx(), true);
    assert!(ok, "{out}");
}

#[test]
fn mutation_tests_do_not_depend_on_known_gaps() {
    let text = std::fs::read_to_string(real().root.join("tests/book_test.rs")).unwrap();
    let start = text.find("fn the_compiled_gate_fails_when_compiled_output_differs").unwrap();
    let body = &text[start..text[start..].find("\n}\n").map(|e| start + e).unwrap()];
    let forbidden = ["known-gaps", "known_gap", "book_repros", "OPEN_QUESTIONS", "?."];
    for f in forbidden {
        assert!(!body.contains(f), "the compiled-gate mutation test must not depend on a compiler gap (found {f:?}); use the runner's test hook");
    }
}

#[test]
fn known_gap_defer_in_a_loop_body_does_not_run_interpreted() {
    let repro = real().root.join("tests/book_repros/defer_in_loop.wj");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_wj")).args(["run", "--interpret"]).arg(&repro).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success() && text.contains("pass 1") && !text.contains("end of pass"), "the interpreter now runs a loop-body defer: delete tests/book_repros/defer_in_loop.wj, this test and its docs/known-gaps.md row\n{text}");
}

// ---------------------------------------------------------------------------------------------
// The feature registry (docs/book/features.toml)
// ---------------------------------------------------------------------------------------------

impl Scratch {
    fn with_registry(&self, features: &str, spec: &str) {
        self.write("docs/book/features.toml", features);
        self.write("docs/spec/widgets.md", spec);
    }
}

const ONE_FEATURE: &str = "[[feature]]\nid = \"widgets\"\ntitle = \"Widgets\"\nfamilies = [\"WID\"]\nspec = \"docs/spec/widgets.md\"\n";
const ONE_RULE: &str = "# Widgets\n\n**SPEC-WID-001** A widget exists.\n";

#[test]
fn a_spec_family_missing_from_the_registry_fails() {
    let s = Scratch::new();
    s.with_registry(ONE_FEATURE, &format!("{ONE_RULE}\n**SPEC-GAD-001** A gadget exists.\n"));
    s.page(&format!("{GOOD}\n<!-- feature: widgets -->\n"));
    assert!(s.fails("SPEC-GAD is in no registry entry"), "{}", show(&s.findings()));
}

#[test]
fn an_unlisted_page_marker_and_a_stale_family_fail() {
    let s = Scratch::new();
    s.with_registry(ONE_FEATURE, ONE_RULE);
    s.page(&format!("{GOOD}\n<!-- feature: widgets -->\n<!-- feature: gizmos -->\n"));
    assert!(s.fails("`<!-- feature: gizmos -->`") || s.fails("gizmos"), "{}", show(&s.findings()));
    s.with_registry(ONE_FEATURE, "# Widgets\n\n**SPEC-OTHER-001** x\n");
    assert!(s.fails("registry names family WID"), "{}", show(&s.findings()));
}

#[test]
fn uncovered_features_are_a_shrink_only_ratchet() {
    let s = Scratch::new();
    s.with_registry(ONE_FEATURE, ONE_RULE);
    assert!(s.fails("features no page covers: 1 exceeds the ratchet 0"), "{}", show(&s.findings()));
    s.write("docs/docs-truth.toml", "[book]\nmax_uncovered_features = 1\n");
    assert_eq!(failures(&s.findings()), 0, "{}", show(&s.findings()));
    s.page(&format!("{GOOD}\n<!-- feature: widgets -->\n"));
    assert!(s.fails("below the ratchet 1"), "{}", show(&s.findings()));
}

#[test]
fn the_side_by_side_prose_claims_hold_for_the_computed_line_counts() {
    // docs/book/src/showcase/side-by-side.md says the Windjammer programs are shorter than the Go ones and no longer than the
    // Rust the compiler generates for them. The numbers come from the page itself, so the sentence cannot go stale.
    let rows = docs_truth::book_showcase::loc_rows(&real(), "showcase/side-by-side.md");
    assert!(rows.len() >= 4, "the page lost its programs: {rows:?}");
    for (task, wj, rs, go, _ts) in rows {
        assert!(wj < go, "{task}: Windjammer ({wj}) is no longer shorter than Go ({go}); fix the prose on the page");
        assert!(wj <= rs, "{task}: Windjammer ({wj}) is now longer than the generated Rust ({rs}); fix the prose on the page");
    }
}

#[test]
fn the_real_registry_lists_every_spec_family() {
    let ctx = real();
    assert!(!docs_truth::book_features::load(&ctx).is_empty(), "docs/book/features.toml is missing");
    let listed: std::collections::BTreeSet<String> = docs_truth::book_features::load(&ctx).into_iter().flat_map(|f| f.families).collect();
    let spec = docs_truth::book_features::spec_families(&ctx);
    assert!(spec.is_subset(&listed), "unlisted spec families: {:?}", spec.difference(&listed).collect::<Vec<_>>());
}

#[test]
fn eval_prints_an_interpolated_string_literal() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_wj")).args(["eval", "\"${6 * 7} ok\""]).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "42 ok", "{text}");
}

// ---------------------------------------------------------------------------------------------
// Companion fences: `rust,generated`, `go,run`, `ts,run`
// ---------------------------------------------------------------------------------------------

const HI: &str = "// expect: hi\nfn main() {\n    println(\"hi\")\n}\n";

#[test]
fn rust_shown_must_be_what_the_compiler_generates() {
    let s = Scratch::new();
    s.page(&format!("```wj\n{HI}```\n\n```rust,generated\nfn main() {{\n    println!(\"hi\");\n}}\n```\n"));
    let (ok, out) = run_samples(&s.ctx(), false);
    assert!(ok, "a correct excerpt passes: {out}");
    s.page(&format!("```wj\n{HI}```\n\n```rust,generated\nfn main() {{\n    println!(\"not what is generated\");\n}}\n```\n"));
    let (ok, out) = run_samples(&s.ctx(), false);
    assert!(!ok && out.contains("the Rust shown is not what the compiler generates") && out.contains("[page_L1]"), "{out}");
}

#[test]
fn go_and_typescript_versions_must_print_the_same_output() {
    let s = Scratch::new();
    let go_ok = "```go,run\npackage main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"hi\")\n}\n```\n";
    let ts_ok = "```ts,run\nconsole.log(\"hi\");\n```\n";
    s.page(&format!("```wj\n{HI}```\n\n{go_ok}\n{ts_ok}"));
    let (ok, out) = run_samples(&s.ctx(), false);
    assert!(ok, "{out}");
    s.page(&format!("```wj\n{HI}```\n\n{}\n{ts_ok}", go_ok.replace("\"hi\"", "\"bye\"")));
    let (ok, out) = run_samples(&s.ctx(), false);
    assert!(!ok && out.contains("the Go version prints different output"), "{out}");
    s.page(&format!("```wj\n{HI}```\n\n{go_ok}\n{}", ts_ok.replace("\"hi\"", "\"bye\"")));
    let (ok, out) = run_samples(&s.ctx(), false);
    assert!(!ok && out.contains("the TypeScript version prints different output"), "{out}");
}

#[test]
fn a_companion_fence_needs_a_program_before_it() {
    let s = Scratch::new();
    s.page("```go,run\npackage main\n\nfunc main() {}\n```\n");
    assert!(s.fails("must follow the Windjammer program"), "{}", show(&s.findings()));
}

#[test]
fn book_regen_rewrites_a_stale_generated_fence_and_refuses_a_broken_program() {
    let s = Scratch::new();
    let stale = format!("```wj\n{HI}```\n\n```rust,generated\nfn main() {{\n    println!(\"stale\");\n}}\n```\n");
    s.page(&stale);
    let (ok, _) = run_samples(&s.ctx(), false);
    assert!(!ok, "a stale excerpt fails before regeneration");
    let wj = std::path::Path::new(env!("CARGO_BIN_EXE_wj"));
    let report = docs_truth::book_regen::regen(&s.ctx(), wj).unwrap();
    assert!(report.contains("-    println!(\"stale\");") && report.contains("+    println!(\"hi\");"), "{report}");
    let (ok, out) = run_samples(&s.ctx(), false);
    assert!(ok, "after regeneration the page passes: {out}");
    assert_eq!(docs_truth::book_regen::regen(&s.ctx(), wj).unwrap(), "every rust,generated fence already matches the compiler\n");
    // a program that does not build: nothing is rewritten and the error says so
    let broken = "```wj\nfn main( {\n```\n\n```rust,generated\nfn main() {}\n```\n";
    s.page(broken);
    let before = std::fs::read_to_string(s.dir.path().join("docs/book/src/page.md")).unwrap();
    let err = docs_truth::book_regen::regen(&s.ctx(), wj).unwrap_err();
    assert!(err.contains("does not build, nothing was rewritten"), "{err}");
    assert_eq!(std::fs::read_to_string(s.dir.path().join("docs/book/src/page.md")).unwrap(), before);
}
