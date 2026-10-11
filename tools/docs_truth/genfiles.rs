//! Registry of generated files and region generators; the only place that maps names to code.

use super::{gen_cli, gen_errors, gen_lang, gen_meta, gen_stdlib, gen_targets, util, Ctx, Finding};
use std::collections::BTreeMap;

pub const HEADER_MARK: &str = "GENERATED FILE - DO NOT EDIT";

/// (path, generator name). Every file under `docs/reference/` must appear here.
pub const FILES: &[(&str, &str)] = &[
    ("docs/reference/cli.md", "cli"),
    ("docs/reference/keywords.md", "keywords"),
    ("docs/reference/intrinsics.md", "intrinsics"),
    ("docs/reference/errors.md", "errors"),
    ("docs/reference/targets.md", "targets"),
    ("docs/reference/stdlib.md", "stdlib"),
    ("docs/generated/metrics.json", "metrics"),
];

fn header(name: &str) -> String {
    format!("<!-- {HEADER_MARK}. Generator: `{name}` (scripts/docs_gen.sh, tools/docs_truth/). Checked by `cargo test --test all docs_truth`. -->\n\n")
}

/// Body of a generator, without header.
pub fn body(ctx: &Ctx, name: &str) -> Result<String, String> {
    Ok(match name {
        "cli" => gen_cli::cli(&ctx.cli),
        "keywords" => gen_lang::keywords(),
        "intrinsics" => gen_lang::intrinsics(),
        "errors" => gen_errors::errors(),
        "targets" => gen_targets::targets(ctx),
        "stdlib" => gen_stdlib::stdlib(ctx),
        "metrics" => gen_meta::metrics_json(ctx),
        _ => return Err(format!("unknown generator `{name}`")),
    })
}

pub fn render_file(ctx: &Ctx, name: &str) -> Result<String, String> {
    let b = body(ctx, name)?;
    Ok(if name == "metrics" { b } else { format!("{}{}", header(name), b.trim_end().to_string() + "\n") })
}

/// Content of a `<!-- wj:gen name=... -->` region.
pub fn region(ctx: &Ctx, name: &str, args: &BTreeMap<String, String>) -> Result<String, String> {
    match name {
        "cli_commands" => Ok(gen_cli::command_table(&ctx.cli)),
        "adr_index" => Ok(gen_meta::adr_index(ctx)),
        "lint_range" => Ok(gen_errors::lint_range()),
        "metric" => gen_meta::metric(ctx, args.get("key").ok_or("metric needs key=")?),
        other if FILES.iter().any(|(_, g)| *g == other) && other != "metrics" => body(ctx, other),
        other => Err(format!("unknown region generator `{other}`")),
    }
}

pub fn write_all(ctx: &Ctx) -> Result<Vec<String>, String> {
    let mut changed = Vec::new();
    for (path, name) in FILES {
        let new = render_file(ctx, name)?;
        let full = ctx.root.join(path);
        if std::fs::read_to_string(&full).ok().as_deref() != Some(new.as_str()) {
            std::fs::create_dir_all(full.parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::write(&full, new).map_err(|e| e.to_string())?;
            changed.push(path.to_string());
        }
    }
    Ok(changed)
}

pub fn check(ctx: &Ctx) -> Vec<Finding> {
    let mut out = Vec::new();
    for (path, name) in FILES {
        match render_file(ctx, name) {
            Err(e) => out.push(Finding::fail("gen", path, 0, e)),
            Ok(want) => {
                let have = ctx.read(path);
                if have.is_empty() {
                    out.push(Finding::fail("gen", path, 0, "generated file missing: run scripts/docs_gen.sh"));
                } else if have != want {
                    let line = have.lines().zip(want.lines()).position(|(a, b)| a != b).map(|i| i + 1).unwrap_or(1);
                    out.push(Finding::fail("gen", path, line, "generated file is stale or hand-edited: run scripts/docs_gen.sh"));
                }
            }
        }
    }
    let registered: Vec<&str> = FILES.iter().map(|(p, _)| *p).collect();
    for f in util::walk(&ctx.root.join("docs")) {
        let rel = format!("docs/{f}");
        if registered.contains(&rel.as_str()) || !(f.ends_with(".md") || f.ends_with(".json")) {
            continue;
        }
        if rel.starts_with("docs/reference/") {
            out.push(Finding::fail("gen", &rel, 1, "hand-written file in docs/reference/ (that directory is generated only; register a generator or move it)"));
        } else if ctx.read(&rel).lines().take(3).any(|l| l.contains(HEADER_MARK)) {
            out.push(Finding::fail("gen", &rel, 1, "carries a GENERATED header but is not in genfiles::FILES"));
        }
    }
    out
}
