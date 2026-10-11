//! Documentation truth (docs/CONTRIBUTING-docs.md): generated files and regions are fresh, doc
//! examples run, links and references resolve, hand-typed stats are ratcheted. The second half
//! mutates a scratch repo to prove each class of drift is actually detected.

#![allow(clippy::duplicate_mod)]

#[path = "../tools/docs_truth/mod.rs"]
mod docs_truth;
#[path = "../src/bin/wj_cli_args.rs"]
mod wj_cli_args;

use clap::CommandFactory;
use docs_truth::{claims, config, ebnf, failures, fences, freshness, genfiles, links, proposal, regions, Ctx, Finding};
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

/// Every `W####` code emitted anywhere in `src/` (linter, parser, CLI diagnostics) is registered in
/// `linter::codes` or the error catalog. Only string literals count (comments may mention codes freely).
#[test]
fn lint_codes_are_registered() {
    let ctx = real();
    let known = docs_truth::gen_errors::known_codes();
    for f in docs_truth::util::walk(&ctx.root.join("src")) {
        if !f.ends_with(".rs") || f == "linter/codes.rs" {
            continue;
        }
        let text = ctx.read(&format!("src/{f}"));
        let code_lines: String = text.lines().filter(|l| !l.trim_start().starts_with("//")).map(|l| format!("{l}\n")).collect();
        for c in links::scan_ids(&code_lines, "W", 4) {
            let quoted = code_lines.contains(&format!("\"{c}")) || code_lines.contains(&format!("`{c}"));
            if quoted {
                assert!(known.contains(&c), "src/{f} emits {c}, which is not in src/linter/codes.rs or the error catalog");
            }
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

#[test]
fn proposals_are_bannered_and_every_proposal_dir_doc_has_a_status() {
    let ctx = real();
    let fs = proposal::check_all(&ctx);
    assert!(fs.is_empty(), "{}", show(&fs));
}

#[test]
fn ebnf_grammar_matches_the_real_lexer_and_parser() {
    let ctx = real();
    let cfg = config::load(&ctx);
    let fs = ebnf::check_all(&ctx, &cfg);
    assert!(fs.is_empty(), "docs/windjammer.ebnf diverges from the parser:\n{}", show(&fs));
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
    s.write("docs/g.md", "<!-- ignore: broken on purpose -->\n```wj,ignore\nfn ( {{ broken\n```\n");
    assert_eq!(failures(&s.all()), 0);
    s.write("docs/g.md", "<!-- should_fail: broken on purpose -->\n```wj,should_fail\nfn ( {{ broken\n```\n");
    assert_eq!(failures(&s.all()), 0);
    s.write("docs/g.md", "<!-- should_fail: claims to fail -->\n```wj,should_fail\nfn main() {}\n```\n");
    assert!(has(&s.all(), "fences", "should_fail"));
    s.write("docs/g.md", "<!-- should_fail: lint demo -->\n```wj,should_fail=W0002\nfn main() {\n    let x = Some(1).unwrap()\n}\n```\n");
    assert_eq!(failures(&s.all()), 0, "{}", show(&s.all()));
    s.write("docs/g.md", "<!-- should_fail: lint demo -->\n```wj,should_fail=W0002\nfn main() {}\n```\n");
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

#[test]
fn proposal_status_marks_commands_as_future_and_requires_a_banner() {
    let s = Scratch::new();
    let body = "# P\n\n> **Proposal: not implemented.** Future work.\n\nRun `wj frobnicate --pgo-auto` (WJ9999).\n\n```wj\nfn ( {{ future syntax\n```\n";
    s.write("docs/proposals/p.md", &format!("---\nstatus: proposal\n---\n{body}"));
    assert_eq!(failures(&s.all()), 0, "{}", show(&s.all()));
    // Without the banner the document is rejected.
    s.write("docs/proposals/p.md", &format!("---\nstatus: proposal\n---\n{}", body.replace("> **Proposal: not implemented.** Future work.", "Future work.")));
    assert!(has(&s.all(), "proposal", "banner"));
    // The same text as a current document: the command, the code and the fence all fail.
    s.write("docs/proposals/p.md", &format!("---\nstatus: current\n---\n{body}"));
    let fs = s.all();
    assert!(has(&fs, "links", "not a subcommand") && has(&fs, "links", "WJ9999") && has(&fs, "fences", ""), "{}", show(&fs));
    // Everything under docs/proposals/ must declare a status, or it would dodge the check by omission.
    s.write("docs/proposals/p.md", body);
    assert!(has(&s.all(), "proposal", "must declare front matter"));
}

#[test]
fn documents_must_verify_their_wj_shell_fences() {
    let s = Scratch::new();
    let toml = s.read("docs/docs-truth.toml");
    s.write("docs/docs-truth.toml", &format!("{toml}\n[verify]\nrequired = [\"README.md\"]\n"));
    s.write("README.md", "```bash\nwj run hello.wj\n```\n");
    assert!(has(&s.all(), "fences", "neither `sh,verify` nor `ignore`"));
    s.write("README.md", "```sh,verify\n$ wj --version\n```\n");
    assert!(!has(&s.all(), "fences", "neither"));
    s.write("README.md", "```bash,ignore\nwj run hello.wj\n```\n```bash\ncargo build --release\n```\n");
    assert!(!has(&s.all(), "fences", "neither"));
}

#[test]
fn sh_verify_supports_heredocs_for_input_files() {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("wj");
    std::fs::write(&bin, "#!/bin/sh
cat \"$2\"\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let ok = "```sh,verify\n$ cat > hello.txt <<'EOF'\nhello there\nEOF\n$ wj run hello.txt\nhello there\n```\n";
    assert!(fences::check_doc("d.md", ok, Some(&bin)).raw.is_empty(), "{:?}", fences::check_doc("d.md", ok, Some(&bin)).raw);
    let bad = ok.replace("hello there\n```", "goodbye\n```");
    assert_eq!(fences::check_doc("d.md", &bad, Some(&bin)).raw.len(), 1);
}

#[test]
fn freshness_front_matter_becomes_required_once_configured() {
    let s = Scratch::new();
    s.write("docs/design/d.md", "# no front matter\n");
    assert_eq!(failures(&s.all()), 0);
    let toml = s.read("docs/docs-truth.toml").replace("tracked = [\"docs/design/*.md\"]", "require = true\ntracked = [\"docs/design/*.md\", \"docs/adr/0*.md\"]");
    s.write("docs/docs-truth.toml", &toml);
    assert!(has(&s.all(), "freshness", "no front matter"));
    s.write("docs/design/d.md", "---\nstatus: current\n---\n# D\n");
    assert!(has(&s.all(), "freshness", "last_verified"));
    // ADR status line and front matter must agree.
    s.write("docs/adr/0001-first.md", "---\nstatus: proposed\n---\n# ADR-0001: First\n\n- **Status:** Accepted (2026-01-01)\n");
    assert!(has(&s.all(), "freshness", "disagrees"));
}

#[test]
fn generated_error_catalog_is_registered_and_checked() {
    let s = Scratch::new();
    assert!(s.read("docs/errors/errors.md").contains("GENERATED FILE"));
    s.write("docs/errors/errors.md", "# hand edited\n");
    assert!(has(&s.all(), "gen", "stale"));
}

fn ebnf_ctx(grammar: &str) -> (Scratch, config::Config) {
    let s = Scratch::new();
    s.write("docs/windjammer.ebnf", grammar);
    let mut cfg = config::Config::default();
    for k in windjammer::lexer::keywords::KEYWORDS {
        cfg.ebnf.unlisted_keywords.insert(k.word.to_string(), "scratch".into());
    }
    for t in ["fn", "let", "struct", "if", "else"] {
        cfg.ebnf.unlisted_keywords.remove(t);
    }
    (s, cfg)
}

const MINI: &str = "program = { item } ;\nitem = function_decl ;\nfunction_decl = \"fn\" IDENT \"(\" \")\" block ;\nblock = \"{\" { statement } \"}\" ;\nstatement = let_stmt | if_stmt ;\nlet_stmt = \"let\" IDENT \"=\" expression ;\nif_stmt = \"if\" expression block [ \"else\" block ] ;\nexpression = INT_LIT | IDENT ;\nstruct_decl = \"struct\" IDENT \"{\" \"}\" ;\nIDENT = LETTER ;\nINT_LIT = DIGIT ;\nLETTER = \"a\" ;\nDIGIT = \"0\" ;\n";

#[test]
fn ebnf_check_accepts_a_grammar_the_parser_accepts() {
    let (s, cfg) = ebnf_ctx(MINI);
    let fs = ebnf::check_all(&s.ctx(), &cfg);
    assert!(fs.is_empty(), "{}", show(&fs));
}

#[test]
fn ebnf_check_detects_structure_vocabulary_and_derivation_drift() {
    // undefined production
    let (s, cfg) = ebnf_ctx(&MINI.replace("expression = INT_LIT | IDENT", "expression = INT_LIT | nonsense"));
    assert!(has(&ebnf::check_all(&s.ctx(), &cfg), "ebnf", "undefined production `nonsense`"));
    // a terminal that is not a lexer keyword
    let (s, cfg) = ebnf_ctx(&MINI.replace("\"let\"", "\"letz\""));
    assert!(has(&ebnf::check_all(&s.ctx(), &cfg), "ebnf", "\"letz\" is not a keyword"));
    // a lexer keyword missing from the grammar (and not listed)
    let (s, mut cfg) = ebnf_ctx(MINI);
    cfg.ebnf.unlisted_keywords.remove("match");
    assert!(has(&ebnf::check_all(&s.ctx(), &cfg), "ebnf", "lexer keyword `match` never appears"));
    // a sentence the parser rejects: `let` needs `=` here, the mutant grammar drops it
    let (s, cfg) = ebnf_ctx(&MINI.replace("\"let\" IDENT \"=\" expression", "\"let\" IDENT expression"));
    let fs = ebnf::check_all(&s.ctx(), &cfg);
    assert!(has(&fs, "ebnf", "let_stmt#0") && has(&fs, "ebnf", "rejected by the parser"), "{}", show(&fs));
    // a listed divergence must be removed once the parser accepts the sentence
    let (s, mut cfg) = ebnf_ctx(MINI);
    cfg.ebnf.divergences.insert("let_stmt#0".into(), "pretend".into());
    assert!(has(&ebnf::check_all(&s.ctx(), &cfg), "ebnf", "remove the entry"));
    // ...and is tolerated while the parser still rejects it: list every rejected sentence of the mutant grammar
    let (s, mut cfg) = ebnf_ctx(&MINI.replace("\"let\" IDENT \"=\" expression", "\"let\" IDENT expression"));
    let rejected: Vec<String> = ebnf::check_all(&s.ctx(), &cfg).iter().filter_map(|f| f.msg.strip_prefix("grammar sentence `").and_then(|m| m.split('`').next()).map(String::from)).collect();
    assert!(rejected.iter().any(|id| id == "let_stmt#0"), "{rejected:?}");
    for id in &rejected {
        cfg.ebnf.divergences.insert(id.clone(), "mutant".into());
    }
    let fs = ebnf::check_all(&s.ctx(), &cfg);
    assert!(fs.is_empty(), "{}", show(&fs));
}

#[allow(dead_code)]
fn _unused(_: &Path) {}

#[test]
fn update_baseline_refuses_to_raise_or_add_baselines() {
    use std::collections::BTreeMap;
    let mut old: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    old.entry("fences".into()).or_default().insert("a.md".into(), 3);
    let mut new: BTreeMap<&'static str, BTreeMap<String, usize>> = BTreeMap::new();
    new.entry("fences").or_default().insert("a.md".into(), 2);
    assert!(config::check_no_increase(&old, &new).is_ok(), "lowering is allowed");
    new.get_mut("fences").unwrap().insert("a.md".into(), 4);
    assert!(config::check_no_increase(&old, &new).unwrap_err().contains("a.md: 4 findings, baseline 3"));
    new.get_mut("fences").unwrap().insert("a.md".into(), 3);
    new.get_mut("fences").unwrap().insert("b.md".into(), 1);
    assert!(config::check_no_increase(&old, &new).unwrap_err().contains("b.md: 1 findings, baseline 0"));
    new.entry("links").or_default().insert("a.md".into(), 1);
    assert!(config::check_no_increase(&old, &new).is_err());
}

/// `wj docs` (the shipped command) must produce exactly the generated, checked-in catalog, and twice the same bytes.
#[test]
fn wj_docs_command_output_is_deterministic_and_matches_the_generated_file() {
    let wj = env!("CARGO_BIN_EXE_wj");
    let run = |dir: &Path| {
        let o = std::process::Command::new(wj).args(["docs", "-f", "markdown", "-o"]).arg(dir).output().unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        std::fs::read_to_string(dir.join("errors.md")).unwrap()
    };
    let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (first, second) = (run(a.path()), run(b.path()));
    assert_eq!(first, second, "wj docs output differs between runs");
    let want = genfiles::body(&real(), "error_catalog").unwrap();
    assert_eq!(first, want, "`wj docs -f markdown` and the registered generator disagree");
}

#[test]
fn ignore_and_should_fail_fences_must_carry_a_reason() {
    let ok = "<!-- ignore: sketch of a future API -->\n```wj,ignore\nfn ( {{\n```\n";
    assert!(fences::check_doc("d.md", ok, None).raw.is_empty());
    let bare = "```wj,ignore\nfn ( {{\n```\n";
    assert_eq!(fences::check_doc("d.md", bare, None).raw.len(), 1);
    let empty = "<!-- ignore: -->\n```wj,ignore\nfn ( {{\n```\n";
    assert_eq!(fences::check_doc("d.md", empty, None).raw.len(), 1);
    let sf = "text\n\n<!-- should_fail: demonstrates the diagnostic -->\n\n```wj,should_fail\nfn ( {{\n```\n";
    assert!(fences::check_doc("d.md", sf, None).raw.is_empty());
    let sf_bare = "text\n```wj,should_fail\nfn ( {{\n```\n";
    assert_eq!(fences::check_doc("d.md", sf_bare, None).raw.len(), 1);
}

#[test]
fn paths_resolve_from_repo_root_or_document_directory_and_negations_exempt_commands() {
    let s = Scratch::new();
    s.write("crates/x/src/server.rs", "");
    s.write("crates/x/README.md", "See `src/server.rs` and `src/nope.rs`.\n");
    let fs = s.all();
    assert!(has(&fs, "links", "src/nope.rs") && !has(&fs, "links", "src/server.rs"), "{}", show(&fs));
    s.write("crates/x/README.md", "See `src/server.rs`.\n");
    s.write("docs/n.md", "- `wj frobnicate` does not exist.\n- `wj frobnicate` is not started.\n");
    assert_eq!(failures(&s.all()), 0, "{}", show(&s.all()));
    s.write("docs/n.md", "- Run `wj frobnicate` to frobnicate.\n");
    assert!(has(&s.all(), "links", "not a subcommand"));
}

#[test]
fn commands_under_planned_or_gap_headings_are_future_not_claims() {
    let s = Scratch::new();
    s.write("docs/p.md", "# P\n\n## Planned (not implemented)\n\n- `wj frobnicate`\n\n## Usage\n\nRun `wj frobnicate`.\n");
    let fs = s.all();
    assert_eq!(fs.iter().filter(|f| f.check == "links" && f.msg.contains("not a subcommand")).count(), 1, "{}", show(&fs));
    assert!(fs.iter().any(|f| f.line == 9 && f.msg.contains("not a subcommand")), "{}", show(&fs));
}

#[test]
fn unexecuted_sh_verify_fences_fail_when_execution_is_required() {
    assert!(fences::not_executed("README.md", 3, false).warn);
    assert!(!fences::not_executed("README.md", 3, true).warn);
    // and without a binary the fence is reported rather than silently passing
    let r = fences::check_doc("d.md", "```sh,verify\n$ wj --version\n```\n", None);
    assert_eq!(r.raw.len(), 1);
}

#[test]
fn ignored_shell_fences_need_a_reason_too() {
    assert_eq!(fences::check_doc("d.md", "```bash,ignore\nwj build\n```\n", None).raw.len(), 1);
    assert!(fences::check_doc("d.md", "<!-- ignore: needs cargo -->\n```bash,ignore\nwj build\n```\n", None).raw.is_empty());
}

#[test]
fn freshness_ignores_docs_that_reference_no_code_paths() {
    // `git diff <sha> HEAD --` with an empty path list used to count the whole repository.
    assert_eq!(freshness::changed_lines(&real(), "HEAD~1", &[]), Some(0));
}
