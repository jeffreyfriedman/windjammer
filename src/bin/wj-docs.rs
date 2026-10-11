//! `wj-docs`: generate and verify documentation (see docs/CONTRIBUTING-docs.md).
//!
//!   wj-docs gen [--check]      rewrite (or verify) generated files and `wj:gen` regions
//!   wj-docs check [NAME..]     run all checks (or: gen regions fences links claims freshness proposal ebnf)
//!   wj-docs update-baseline    lower ratchet baselines to the current counts; refuses to raise or add any
//!   wj-docs stamp FILE [STATUS]  set front matter `last_verified` to HEAD

use clap::CommandFactory;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[path = "../../tools/docs_truth/mod.rs"]
mod docs_truth;
#[path = "wj_cli_args.rs"]
mod wj_cli_args;

use docs_truth::{claims, config, ebnf, failures, fences, freshness, genfiles, links, proposal, regions, Ctx, Finding};

fn main() {
    std::panic::set_hook(Box::new(|_| {})); // the parser/lexer panic on junk snippets; checks catch it
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ctx = Ctx { root, cli: wj_cli_args::Cli::command() };
    let code = match args.first().map(|s| s.as_str()) {
        Some("gen") => gen(&ctx, args.iter().any(|a| a == "--check")),
        Some("check") => check(&ctx, &args[1..]),
        Some("update-baseline") => update_baseline(&ctx),
        Some("stamp") if args.len() >= 2 => match freshness::stamp(&ctx, &args[1], args.get(2).map(|s| s.as_str()).unwrap_or("current")) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{e}");
                1
            }
        },
        _ => {
            eprintln!("usage: wj-docs gen [--check] | check [gen|regions|fences|links|claims|freshness|proposal|ebnf].. | update-baseline | stamp FILE [STATUS]");
            2
        }
    };
    std::process::exit(code);
}

fn report(fs: &[Finding]) -> i32 {
    for f in fs {
        eprintln!("{}", f.render());
    }
    let n = failures(fs);
    eprintln!("{n} failure(s), {} warning(s)", fs.len() - n);
    (n > 0) as i32
}

fn gen(ctx: &Ctx, check_only: bool) -> i32 {
    if check_only {
        let cfg = config::load(ctx);
        let mut fs = genfiles::check(ctx);
        fs.extend(regions::check_all(ctx, &cfg));
        return report(&fs);
    }
    let cfg = config::load(ctx);
    let mut changed = match genfiles::write_all(ctx) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    match regions::write_all(ctx, &cfg) {
        Ok(c) => changed.extend(c),
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    }
    for c in &changed {
        println!("updated {c}");
    }
    0
}

fn check(ctx: &Ctx, which: &[String]) -> i32 {
    if which.is_empty() {
        return report(&docs_truth::run_all(ctx));
    }
    let cfg = config::load(ctx);
    let mut fs = Vec::new();
    for w in which {
        match w.as_str() {
            "gen" => fs.extend(genfiles::check(ctx)),
            "regions" => fs.extend(regions::check_all(ctx, &cfg)),
            "fences" => fs.extend(fences::check_all(ctx, &cfg)),
            "links" => fs.extend(links::check_all(ctx, &cfg)),
            "claims" => fs.extend(claims::check_all(ctx, &cfg)),
            "freshness" => fs.extend(freshness::check_all(ctx, &cfg)),
            "proposal" => fs.extend(proposal::check_all(ctx)),
            "ebnf" => fs.extend(ebnf::check_all(ctx, &cfg)),
            other => {
                eprintln!("unknown check {other}");
                return 2;
            }
        }
    }
    report(&fs)
}

fn update_baseline(ctx: &Ctx) -> i32 {
    let cfg = config::load(ctx);
    let mut counts: BTreeMap<&'static str, BTreeMap<String, usize>> = BTreeMap::new();
    counts.insert("claims", claims::raw_counts(ctx, &cfg));
    counts.insert("fences", fences::raw_counts(ctx, &cfg));
    counts.insert("links", links::raw_counts(ctx, &cfg));
    if let Err(e) = config::check_no_increase(&cfg.baseline, &counts) {
        eprintln!("refusing to update baselines (they are ratchets: fix the documents, never raise a baseline):\n{e}");
        return 1;
    }
    let text = ctx.read(config::PATH);
    let new = format!("{}{}", config::head_without_baseline(&text), config::render_baseline(&counts));
    std::fs::write(ctx.root.join(config::PATH), new).expect("write config");
    println!("updated {}", config::PATH);
    0
}
