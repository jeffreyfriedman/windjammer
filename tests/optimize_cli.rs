//! `wj optimize` pass table (src/cli/optimize.rs) agrees with the optimizer and with docs/optimization/PASSES.md:
//! every pass has a soundness section whose heading produces the anchor the command prints, and the default state
//! it reports is `OptimizationConfig::production()`'s. A pass added without a soundness section fails here.

use windjammer::cli::optimize::PASSES;
use windjammer::optimizer::OptimizationConfig;

/// GitHub-style anchor of a markdown heading: lowercase, spaces to `-`, punctuation other than `-`/`_` dropped.
fn anchor(heading: &str) -> String {
    let mut out = String::from("#");
    for c in heading.trim().to_lowercase().chars() {
        if c == ' ' {
            out.push('-');
        } else if c.is_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
        }
    }
    out
}

fn production_default(name: &str) -> bool {
    let c = OptimizationConfig::production();
    match name {
        "temp_clone" => c.temp_clone,
        "copy_clone" => c.copy_clone,
        "macro_arg_clone" => c.macro_arg_clone,
        "same_type_cast" => c.same_type_cast,
        "with_capacity" => c.with_capacity,
        "string_append" => c.string_append,
        "inline_policy" => c.inline_policy,
        "dead_helpers" => c.dead_helpers,
        "scratch_reuse" => c.scratch_reuse,
        other => panic!("pass `{other}` in the wj optimize table is not an OptimizationConfig switch"),
    }
}

#[test]
fn every_pass_has_a_soundness_section_and_the_true_default() {
    let doc = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/optimization/PASSES.md")).unwrap();
    let anchors: Vec<String> = doc.lines().filter_map(|l| l.strip_prefix("## ")).map(anchor).collect();
    assert_eq!(PASSES.len(), 9, "the optimizer has nine switches; update the wj optimize table with the new pass");
    for p in PASSES {
        assert!(anchors.contains(&p.soundness.to_string()), "{}: no PASSES.md heading with anchor {} (have {:?})", p.name, p.soundness, anchors);
        assert_eq!(p.default_on, production_default(p.name), "{}: wj optimize reports the wrong default", p.name);
    }
}

#[test]
fn anchor_follows_github_rules() {
    assert_eq!(anchor("P8 scratch_reuse (OFF; auto-arena, ADR-0025)"), "#p8-scratch_reuse-off-auto-arena-adr-0025");
    assert_eq!(anchor("P5 string_append (ON)"), "#p5-string_append-on");
}
