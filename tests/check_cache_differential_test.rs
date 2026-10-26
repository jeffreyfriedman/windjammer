//! The check caches (whole-run result cache and per-file analysis memo, `src/check/{cache,file_cache}.rs`) never
//! change diagnostics: cached and uncached runs agree on diagnostics, success and the `wj run` gate decision, on
//! generated projects under random dependency edits and on the repository's example programs.
//! Run: cargo test --release --test check_cache_differential

#[path = "common/check_cache/generated.rs"]
mod generated;
#[path = "common/check_cache/support.rs"]
mod support;
#[path = "common/check_cache/effects.rs"]
mod effects;

use generated::Project;
use std::path::Path;
use support::{compare, diags, in_dir, report, write, Rng};

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let owned: Vec<(String, String)> = files.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
    write(d.path(), &owned);
    d
}

const TOML: &str = "[package]\nname = \"p\"\nversion = \"0.1.0\"\n";
const MAIN: &str = "use crate::dep\n\nfn main() {\n    let x = dep::f(1)\n    println(\"${x}\")\n}\n";
const DEP: &str = "pub fn f(x: int) -> int {\n    x + 1\n}\n";

/// Editing a dependency (never the checked file itself) changes the diagnostics of the checked entry exactly as an
/// uncached run does. With a cache key that omits the dependency's content this fails (ADR-0020 round 3 rung 4).
#[test]
fn dependency_edit_changes_diagnostics_like_uncached() {
    let d = project(&[("wj.toml", TOML), ("src/main.wj", MAIN), ("src/dep.wj", DEP)]);
    let main = d.path().join("src/main.wj");
    in_dir(d.path(), || {
        let before = compare(&main, "clean project");
        assert!(before.repeat_hit, "the repeat run is a whole-run cache hit");
        assert!(before.uncached.success);
        write(d.path(), &[("src/dep.wj".into(), DEP.replace("(x: int)", "(x: int, y: int)"))]);
        let after = compare(&main, "dep::f gained a parameter");
        assert!(!after.uncached.success, "an arity mismatch in main.wj is an error");
        assert_ne!(diags(&before.uncached), diags(&after.uncached), "the edit changes the diagnostics");
        write(d.path(), &[("src/dep.wj".into(), DEP.replace("pub fn", "fn"))]);
        let private = compare(&main, "dep::f made private");
        assert!(!private.uncached.success);
        write(d.path(), &[("src/dep.wj".into(), DEP.into())]);
        assert!(compare(&main, "reverted").uncached.success);
    });
}

/// The per-file memo is live: an unchanged file's analysis comes from it (a planted entry shows up), and a
/// changed file's entry is ignored (the planted entry disappears).
#[test]
fn per_file_memo_is_used_for_unchanged_files_only() {
    let d = project(&[("wj.toml", TOML), ("src/main.wj", MAIN), ("src/dep.wj", DEP)]);
    let main = d.path().join("src/main.wj");
    in_dir(d.path(), || {
        assert!(report(&main, true).success);
        let memo = std::fs::read_dir(d.path().join(".wj-cache"))
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .find(|p| p.file_name().unwrap().to_string_lossy().starts_with("check-files-"))
            .expect("memo file written under .wj-cache");
        let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&memo).unwrap()).unwrap();
        let planted = windjammer::diagnostics::Diagnostic::error("WJ9999", "planted memo entry");
        let key = main.display().to_string();
        v["entries"][&key][1].as_array_mut().expect("main.wj entry").push(serde_json::to_value(&planted).unwrap());
        std::fs::write(&memo, v.to_string()).unwrap();
        write(d.path(), &[("src/dep.wj".into(), format!("{DEP}// touched\n"))]);
        assert!(diags(&report(&main, true)).contains("WJ9999"), "unchanged main.wj: analysis read from the memo");
        write(d.path(), &[("src/main.wj".into(), format!("{MAIN}// touched\n"))]);
        assert!(!diags(&report(&main, true)).contains("WJ9999"), "changed main.wj: memo entry ignored");
    });
}

/// A `.wj.meta` sidecar the numeric inference may read (relative to the working directory) is a cache input.
#[test]
fn new_meta_sidecar_invalidates_the_cache() {
    let d = project(&[("wj.toml", TOML), ("src/main.wj", MAIN), ("src/dep.wj", DEP)]);
    let main = d.path().join("src/main.wj");
    in_dir(d.path(), || {
        report(&main, true);
        assert!(report(&main, true).summary.files_cached > 0, "warm");
        write(d.path(), &[(".wj-cache/dep.wj.meta".into(), "{}".into())]);
        assert_eq!(report(&main, true).summary.files_cached, 0, "a new sidecar is a miss");
        assert!(report(&main, true).summary.files_cached > 0, "and then warm again");
    });
}

/// Random dependency edits on generated projects: cached == uncached at every step, for the entry file and the
/// project directory.
#[test]
fn generated_projects_under_random_edits() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let (mut steps, mut hits, mut failing, mut analysis) = (0, 0, 0, 0);
    for case in 0..24 {
        let original = Project::generate(&mut rng);
        let mut cur = original.clone();
        let d = tempfile::tempdir().unwrap();
        write(d.path(), &cur.render(&original));
        in_dir(d.path(), || {
            let mut log = vec!["generated".to_string()];
            for _ in 0..6 {
                for target in [d.path().join("src/main.wj"), d.path().to_path_buf()] {
                    let c = compare(&target, &format!("case {case}: {}", log.join(" / ")));
                    steps += 1;
                    hits += c.repeat_hit as usize;
                    failing += !c.uncached.success as usize;
                    analysis += c.uncached.diagnostics.iter().any(|d| d.code == "WJ0027") as usize;
                }
                log.push(cur.edit(&original, &mut rng));
                write(d.path(), &cur.render(&original));
            }
        });
    }
    eprintln!("generated: {steps} comparisons, {hits} repeat hits, {failing} failing, {analysis} with analysis-phase findings");
    assert!(analysis >= 10, "analysis-phase findings (what the per-file memo stores) must occur: {analysis}");
    assert!(hits * 10 >= steps * 9, "the cache must actually be exercised: {hits}/{steps} repeat hits");
    assert!(failing * 5 >= steps && failing * 5 <= steps * 4, "a mix of clean and failing states: {failing}/{steps}");
}

/// The repository's example programs (passing) and check-negative mutants (failing), each as a project with that
/// program as `src/main.wj`, then randomly edited (lines dropped or duplicated): cached == uncached at every step.
/// (One program per project: several programs in one directory define colliding names, and `wj check` then reports
/// some diagnostics at a nondeterministic file; see ADR-0020 round 3 rung 4.)
#[test]
fn real_programs_under_random_edits() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut pool: Vec<std::path::PathBuf> = Vec::new();
    for dir in ["examples/verified", "tests/check_negative/mutants"] {
        let mut v: Vec<_> = std::fs::read_dir(repo.join(dir)).unwrap().flatten().map(|e| e.path()).collect();
        v.retain(|p| p.extension().is_some_and(|e| e == "wj"));
        v.sort();
        pool.extend(v);
    }
    assert!(pool.len() > 100, "corpus present: {}", pool.len());
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    let (mut steps, mut failing, mut hits) = (0, 0, 0);
    for p in &pool {
        let d = tempfile::tempdir().unwrap();
        let mut text = std::fs::read_to_string(p).unwrap();
        write(d.path(), &[("wj.toml".into(), TOML.into()), ("src/main.wj".into(), text.clone())]);
        let main = d.path().join("src/main.wj");
        in_dir(d.path(), || {
            for round in 0..2 {
                let c = compare(&main, &format!("{}, round {round}", p.display()));
                steps += 1;
                hits += c.repeat_hit as usize;
                failing += !c.uncached.success as usize;
                let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
                if !lines.is_empty() {
                    let at = rng.below(lines.len());
                    if rng.below(2) == 0 {
                        lines.remove(at);
                    } else {
                        let l = lines[at].clone();
                        lines.insert(at, l);
                    }
                }
                text = lines.join("\n") + "\n";
                write(d.path(), &[("src/main.wj".into(), text.clone())]);
            }
        });
    }
    eprintln!("real programs: {} programs, {steps} comparisons, {hits} repeat hits, {failing} failing", pool.len());
    assert!(hits * 10 >= steps * 9, "the cache must actually be exercised: {hits}/{steps} repeat hits");
    assert!(failing > 10 && failing < steps, "{steps} steps, {failing} failing");
}
