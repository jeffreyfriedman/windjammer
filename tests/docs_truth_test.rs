//! Documentation truth (docs/CONTRIBUTING-docs.md): generated files and regions are fresh, doc
//! examples run, links and references resolve, hand-typed stats are ratcheted. The second half
//! mutates a scratch repo to prove each class of drift is actually detected.

#![allow(clippy::duplicate_mod)]

#[path = "../tools/docs_truth/mod.rs"]
mod docs_truth;
#[path = "../src/bin/wj_cli_args.rs"]
mod wj_cli_args;

use clap::CommandFactory;
use docs_truth::{claims, config, failures, fences, freshness, genfiles, links, regions, Ctx, Finding};
use std::path::{Path, PathBuf};

fn real() -> Ctx {
    Ctx { root: PathBuf::from(env!("CARGO_MANIFEST_DIR")), cli: wj_cli_args::Cli::command() }
}

fn show(fs: &[Finding]) -> String {
    fs.iter().map(|f| f.render()).collect::<Vec<_>>().join("\n")
}

// ---------------------------------------------------------------------------------------------
// The real repository
// ---------------------------------------------------------------------------------------------

#[test]
fn generated_files_and_regions_are_fresh() {
    let ctx = real();
    let cfg = config::load(&ctx);
    let mut fs = genfiles::check(&ctx);
    fs.extend(regions::check_all(&ctx, &cfg));
    assert_eq!(failures(&fs), 0, "run scripts/docs_gen.sh\n{}", show(&fs));
}

#[test]
fn doc_fences_run() {
    let ctx = real();
    let fs = fences::check_all(&ctx, &config::load(&ctx));
    assert_eq!(failures(&fs), 0, "\n{}", show(&fs));
}

#[test]
fn doc_links_and_references_resolve() {
    let ctx = real();
    let fs = links::check_all(&ctx, &config::load(&ctx));
    assert_eq!(failures(&fs), 0, "\n{}", show(&fs));
}

#[test]
fn no_new_hand_typed_statistics() {
    let ctx = real();
    let fs = claims::check_all(&ctx, &config::load(&ctx));
    assert_eq!(failures(&fs), 0, "\n{}", show(&fs));
}

#[test]
fn freshness_metadata_is_well_formed() {
    let ctx = real();
    let fs = freshness::check_all(&ctx, &config::load(&ctx));
    assert_eq!(failures(&fs), 0, "\n{}", show(&fs));
}

/// Every `"W####"` literal in the linter sources is registered in `linter::codes`.
#[test]
fn lint_codes_are_registered() {
    let ctx = real();
    let known = docs_truth::gen_errors::known_codes();
    for f in docs_truth::util::walk(&ctx.root.join("src/linter")) {
        if !f.ends_with(".rs") || f == "codes.rs" {
            continue;
        }
        for c in links::scan_ids(&ctx.read(&format!("src/linter/{f}")), "W", 4) {
            assert!(known.contains(&c), "src/linter/{f} uses {c}, which is not in src/linter/codes.rs or the error catalog");
        }
    }
}

#[test]
fn readme_quickstart_commands_exist_in_cli() {
    let ctx = real();
    for cmd in ["wj new demo", "wj build --release", "wj run --interpret"] {
        assert!(links::check_doc(&ctx, &config::Config::default(), "x.md", &format!("`{cmd} f.wj`"), &Default::default(), &Default::default(), &[], &Default::default()).iter().all(|f| !f.msg.contains("not a subcommand")), "{cmd}");
    }
}

// ---------------------------------------------------------------------------------------------
// Mutation self-tests on a scratch repository
// ---------------------------------------------------------------------------------------------

struct Scratch {
    dir: tempfile::TempDir,
}

impl Scratch {
    fn new() -> Scratch {
        let s = Scratch { dir: tempfile::tempdir().unwrap() };
        s.write("docs/docs-truth.toml", "[freshness]\ntracked = [\"docs/design/*.md\"]\n");
        s.write("docs/adr/0001-first.md", "# ADR-0001: First\n\n- **Status:** Accepted (2026-01-01)\n");
        s.write("tests/conformance_programs/SUPPORT.toml", "[[unsupported]]\nprogram = \"01_a\"\nbackend = \"go\"\nreason = \"x: y\"\n");
        s.write("tests/conformance_programs/01_a.wj", "fn main() {}\n");
        s.write("tests/common/conformance/backends.rs", "pub const ALL_BACKENDS: [Backend; 2] = [Backend::Rust, Backend::Go];\n");
        s.write("std/a.wj", "/// Adds.\nfn add(a: int, b: int) -> int {\n    a + b\n}\n");
        s.write("src/real.rs", "");
        // paths cited by the generated headers must exist, as in the real repo
        for f in ["src/bin/wj_cli_args.rs", "src/error_codes.rs", "src/error_catalog.rs", "src/linter/codes.rs", "src/intrinsics/mod.rs", "src/lexer/keywords.rs", "docs/adr/0014-surface.md"] {
            s.write(f, "");
        }
        let ctx = s.ctx();
        genfiles::write_all(&ctx).unwrap();
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
    fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.dir.path().join(rel)).unwrap()
    }
    fn all(&self) -> Vec<Finding> {
        docs_truth::run_all(&self.ctx())
    }
}

fn has(fs: &[Finding], check: &str, needle: &str) -> bool {
    fs.iter().any(|f| !f.warn && f.check == check && f.msg.contains(needle))
}

#[test]
fn scratch_baseline_is_clean() {
    let s = Scratch::new();
    let fs = s.all();
    assert_eq!(failures(&fs), 0, "{}", show(&fs));
}

#[test]
fn detects_drifted_generated_file() {
    let s = Scratch::new();
    let p = "docs/reference/keywords.md";
    s.write(p, &format!("{}\nhand edit\n", s.read(p)));
    assert!(has(&s.all(), "gen", "stale or hand-edited"));
}

#[test]
fn detects_missing_generated_file_and_foreign_files() {
    let s = Scratch::new();
    std::fs::remove_file(s.dir.path().join("docs/reference/cli.md")).unwrap();
    assert!(has(&s.all(), "gen", "missing"));
    let s = Scratch::new();
    s.write("docs/reference/notes.md", "# hand written\n");
    assert!(has(&s.all(), "gen", "hand-written file in docs/reference"));
}

#[test]
fn detects_cli_flag_change_without_regeneration() {
    let s = Scratch::new();
    let mut ctx = s.ctx();
    ctx.cli = ctx.cli.mut_subcommand("build", |c| c.arg(clap::Arg::new("zz").long("zz-new-flag")));
    assert!(has(&genfiles::check(&ctx), "gen", "stale"), "new flag must make cli.md stale");
    // and a doc using a flag that does not exist is caught
    s.write("README.md", "Run `wj build --no-such-flag x.wj`.\n");
    assert!(has(&s.all(), "links", "no flag `--no-such-flag`"));
    s.write("README.md", "Run `wj frobnicate x.wj`.\n");
    assert!(has(&s.all(), "links", "not a subcommand"));
}

#[test]
fn detects_stale_region_and_repairs_it() {
    let s = Scratch::new();
    s.write("README.md", "# T\n<!-- wj:gen name=metric key=adr.count -->\n7\n<!-- wj:end -->\n");
    assert!(has(&s.all(), "regions", "stale"));
    regions::write_all(&s.ctx(), &config::load(&s.ctx())).unwrap();
    assert!(s.read("README.md").contains("\n2\n"));
    assert_eq!(failures(&s.all()), 0);
    s.write("README.md", "<!-- wj:gen name=nope -->\n<!-- wj:end -->\n");
    assert!(has(&s.all(), "regions", "unknown region generator"));
    s.write("README.md", "<!-- wj:gen name=adr_index -->\n");
    assert!(has(&s.all(), "regions", "no wj:end"));
}

#[test]
fn detects_broken_and_checks_annotated_examples() {
    let s = Scratch::new();
    s.write("docs/g.md", "```wj\nfn main() { println(\"hi\") }\n```\n");
    assert_eq!(failures(&s.all()), 0, "{}", show(&s.all()));
    s.write("docs/g.md", "```wj\nfn ( {{ broken\n```\n");
    assert!(has(&s.all(), "fences", "parse error"));
    s.write("docs/g.md", "```wj,ignore\nfn ( {{ broken\n```\n");
    assert_eq!(failures(&s.all()), 0);
    s.write("docs/g.md", "```wj,should_fail\nfn ( {{ broken\n```\n");
    assert_eq!(failures(&s.all()), 0);
    s.write("docs/g.md", "```wj,should_fail\nfn main() {}\n```\n");
    assert!(has(&s.all(), "fences", "should_fail"));
    s.write("docs/g.md", "```wj,should_fail=W0002\nfn main() {\n    let x = Some(1).unwrap()\n}\n```\n");
    assert_eq!(failures(&s.all()), 0, "{}", show(&s.all()));
    s.write("docs/g.md", "```wj,should_fail=W0002\nfn main() {}\n```\n");
    assert!(has(&s.all(), "fences", "W0002"));
}

#[test]
fn expect_markers_run_in_the_interpreter() {
    let s = Scratch::new();
    s.write("docs/g.md", "```wj\nfn main() {\n    println(\"hi ${1 + 2}\")\n}\n// expect: hi 3\n```\n");
    assert_eq!(failures(&s.all()), 0, "{}", show(&s.all()));
    s.write("docs/g.md", "```wj\nfn main() {\n    println(\"hi ${1 + 2}\")\n}\n// expect: hi 4\n```\n");
    assert!(has(&s.all(), "fences", "expected"));
}

#[test]
fn rustdoc_fences_are_checked() {
    let s = Scratch::new();
    s.write("src/doc.rs", "//! Intro\n//!\n//! ```windjammer\n//! fn ( {{ nope\n//! ```\n");
    let fs = s.all();
    assert!(fs.iter().any(|f| !f.warn && f.check == "fences" && f.file == "src/doc.rs" && f.line == 3), "{}", show(&fs));
}

#[test]
fn sh_verify_executes_and_compares() {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("wj");
    std::fs::write(&bin, "#!/bin/sh\necho \"wj says $1\"\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let ok = "```sh,verify\n$ wj hello\nwj says hello\n```\n";
    assert!(fences::check_doc("d.md", ok, Some(&bin)).raw.is_empty());
    let bad = "```sh,verify\n$ wj hello\nwj says bye\n```\n";
    assert_eq!(fences::check_doc("d.md", bad, Some(&bin)).raw.len(), 1);
    let fail = "```sh,verify\n$ false\n```\n";
    assert_eq!(fences::check_doc("d.md", fail, Some(&bin)).raw.len(), 1);
}

#[test]
fn detects_dead_links_paths_and_ids() {
    let s = Scratch::new();
    s.write("docs/a.md", "# A\n\nSee [b](b.md) and [x](#nope).\n");
    let fs = s.all();
    assert!(has(&fs, "links", "dead link"));
    assert!(has(&fs, "links", "dead anchor"));
    s.write("docs/a.md", "# A\n\nSee [c](adr/0001-first.md#adr-0001-first) and `src/real.rs` and `src/gone.rs:12`.\n");
    let fs = s.all();
    assert!(has(&fs, "links", "src/gone.rs"));
    assert!(!has(&fs, "links", "src/real.rs"));
    assert!(!has(&fs, "links", "0001-first"));
    s.write("docs/a.md", "ADR-0099 and WJ9999 and W0002 and WJ0001.\n");
    let fs = s.all();
    assert!(has(&fs, "links", "ADR-0099"));
    assert!(has(&fs, "links", "WJ9999"));
    assert!(!has(&fs, "links", "W0002"));
    assert!(!has(&fs, "links", "WJ0001"));
}

#[test]
fn rule_id_namespaces_are_pluggable() {
    let s = Scratch::new();
    s.write("docs/docs-truth.toml", "[[rule_ids]]\nprefix = \"SPEC-\"\ndefs = \"docs/spec/**/*.md\"\n");
    s.write("docs/a.md", "Per SPEC-OWN-01 the compiler decides.\n");
    assert_eq!(failures(&s.all()), 0, "inactive until defs exist");
    s.write("docs/spec/own.md", "## SPEC-OWN-01 Ownership\n");
    assert_eq!(failures(&s.all()), 0);
    s.write("docs/a.md", "Per SPEC-OWN-99 the compiler decides.\n");
    assert!(has(&s.all(), "links", "SPEC-OWN-99"));
}

#[test]
fn removed_files_flagged_unless_documented_as_removed() {
    let s = Scratch::new();
    s.write("docs/a.md", "The `scripts/old.sh` helper runs it.\n");
    assert!(has(&s.all(), "links", "scripts/old.sh"));
}

#[test]
fn detects_hand_typed_statistics_with_ratchet() {
    let s = Scratch::new();
    s.write("docs/a.md", "We pass 1,234 tests and 69 / 90 conformance programs.\n");
    let fs = s.all();
    assert!(has(&fs, "claims", "hand-typed statistic"));
    s.write("docs/docs-truth.toml", "[baseline.claims]\n\"docs/a.md\" = 3\n");
    assert_eq!(failures(&s.all()), 0, "baseline tolerates existing numbers");
    s.write("docs/a.md", "We pass 1,234 tests and 69 / 90 conformance programs and 55 keywords.\n");
    assert!(has(&s.all(), "claims", "baseline allows 3"), "a new hand-typed number must fail");
    s.write("docs/a.md", "We pass <!-- wj:gen name=metric key=adr.count --> tests.\n");
    assert_eq!(failures(&s.all()), 0, "{}", show(&s.all()));
}

#[test]
fn freshness_flags_bad_metadata_and_stale_docs() {
    let s = Scratch::new();
    s.write("docs/design/d.md", "# no front matter\n");
    let fs = s.all();
    assert!(fs.iter().any(|f| f.warn && f.check == "freshness"));
    assert_eq!(failures(&fs), 0);
    s.write("docs/design/d.md", "---\nstatus: bogus\nlast_verified: abc\n---\n# D\n");
    assert!(has(&s.all(), "freshness", "unknown status"));
    s.write("docs/design/d.md", "---\nstatus: current\n---\n# D\n");
    assert_eq!(failures(&s.all()), 0);
}

#[test]
fn keywords_and_operators_come_from_the_lexer() {
    let kw = docs_truth::gen_lang::keywords();
    assert!(kw.contains("`fn`") && kw.contains("PipeOp") && kw.contains("`..=`"));
    let ops = docs_truth::gen_lang::operators();
    assert!(ops.len() > 25, "{}", ops.len());
}

#[allow(dead_code)]
fn _unused(_: &Path) {}
