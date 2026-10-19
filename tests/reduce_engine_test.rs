//! `wj reduce` library-level tests with an in-process oracle: determinism, resumability, and a
//! mutation test showing the planted-construct assertion catches a reducer that drops needed code.

#[path = "common/reduce_fixtures.rs"]
mod fx;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use windjammer::reduce::{run, Oracle, Project, ReduceOptions};

fn has_plant(p: &Project) -> bool {
    p.files
        .values()
        .any(|t| t.contains("* 7919") || t.contains("*7919"))
}

fn opts(src: &Path, out: &Path, oracle: Oracle) -> ReduceOptions {
    ReduceOptions {
        input: src.to_path_buf(),
        oracle,
        out: out.to_path_buf(),
        jobs: 2,
        seed: 3,
        max_steps: 100_000,
        time_budget: Duration::from_secs(600),
        candidate_timeout: Duration::from_secs(60),
        keep_names: false,
        wj_bin: fx::wj(),
        target_base: None,
        verbose: false,
    }
}

fn plant_oracle() -> Oracle {
    Oracle::Predicate(Arc::new(has_plant))
}

/// The independent assertion every reducer result must satisfy.
fn assert_planted_construct_survives(out: &Path) {
    let tree = fx::read_tree(out);
    assert!(
        tree.iter()
            .any(|(p, t)| p.ends_with(".wj") && (t.contains("* 7919") || t.contains("*7919"))),
        "planted construct was dropped"
    );
}

fn tree_without_state(out: &Path) -> Vec<(String, String)> {
    fx::read_tree(out)
}

#[test]
fn reduces_to_the_planted_construct() {
    let d = tempfile::tempdir().unwrap();
    let (src, out) = (d.path().join("src"), d.path().join("out"));
    fx::write_program(&src, &fx::synthetic_program(30, Some(11)));
    let o = run(&opts(&src, &out, plant_oracle())).unwrap();
    assert_eq!(o.files_after, 1);
    assert!(o.lines_after <= 8, "{o:?}");
    assert!(o.anonymized);
    assert_planted_construct_survives(&out);
}

#[test]
#[should_panic(expected = "planted construct was dropped")]
fn mutation_a_reducer_that_drops_needed_code_fails_the_assertion() {
    // A reducer whose oracle accepts everything deletes the planted construct.
    let d = tempfile::tempdir().unwrap();
    let (src, out) = (d.path().join("src"), d.path().join("out"));
    fx::write_program(&src, &fx::synthetic_program(6, Some(2)));
    // the baseline must hold for `run` to start, so the broken oracle is true only after the first call
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let c2 = calls.clone();
    let broken = Oracle::Predicate(Arc::new(move |p: &Project| {
        c2.fetch_add(1, std::sync::atomic::Ordering::SeqCst) > 0 || has_plant(p)
    }));
    let _ = run(&opts(&src, &out, broken));
    assert_planted_construct_survives(&out);
}

#[test]
fn same_seed_gives_identical_output_regardless_of_jobs() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("src");
    fx::write_program(&src, &fx::synthetic_program(20, Some(7)));
    let mut trees = Vec::new();
    for (i, jobs) in [1usize, 1, 3].into_iter().enumerate() {
        let out = d.path().join(format!("out{i}"));
        let mut o = opts(&src, &out, plant_oracle());
        o.jobs = jobs;
        run(&o).unwrap();
        trees.push(
            tree_without_state(&out)
                .into_iter()
                .filter(|(p, _)| p != "repro.md")
                .collect::<Vec<_>>(),
        );
    }
    assert_eq!(trees[0], trees[1], "same seed, same jobs");
    assert_eq!(trees[0], trees[2], "jobs must not change the result");
}

#[test]
fn resumes_from_the_state_file_after_a_budget_stop() {
    let d = tempfile::tempdir().unwrap();
    let (src, out) = (d.path().join("src"), d.path().join("out"));
    fx::write_program(&src, &fx::synthetic_program(25, Some(9)));
    let mut first = opts(&src, &out, plant_oracle());
    first.max_steps = 5;
    let a = run(&first).unwrap();
    assert!(a.lines_after > 8, "budget stop leaves work to do: {a:?}");
    let b = run(&opts(&src, &out, plant_oracle())).unwrap();
    assert!(b.lines_after <= 8, "{b:?}");
    assert_planted_construct_survives(&out);
}
