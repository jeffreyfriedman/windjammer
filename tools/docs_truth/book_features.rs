//! The registry of Windjammer's language features (`docs/book/features.toml`) and its gates.
//!
//! * every `**SPEC-FAMILY-nnn**` rule family in `docs/spec/*.md` must belong to some registry entry (no unlisted features);
//! * every entry must be taught by a book page that carries `<!-- feature: ID -->`; the number of entries no page covers
//!   is a shrink-only ratchet (`[book] max_uncovered_features`);
//! * entries name their spec chapter, which must exist.

use super::config::Config;
use super::{book, util, Ctx, Finding};
use std::collections::BTreeSet;

pub const REGISTRY: &str = "docs/book/features.toml";

#[derive(Debug, Clone)]
pub struct Feature {
    pub id: String,
    pub title: String,
    pub families: Vec<String>,
    pub spec: Option<String>,
}

pub fn load(ctx: &Ctx) -> Vec<Feature> {
    let text = ctx.read(REGISTRY);
    if text.is_empty() {
        return vec![];
    }
    let v: toml::Value = toml::from_str(&text).unwrap_or_else(|e| panic!("{REGISTRY}: {e}"));
    let strs = |x: Option<&toml::Value>| -> Vec<String> { x.and_then(|a| a.as_array()).into_iter().flatten().filter_map(|s| s.as_str().map(String::from)).collect() };
    v.get("feature")
        .and_then(|f| f.as_array())
        .into_iter()
        .flatten()
        .map(|f| Feature {
            id: f.get("id").and_then(|s| s.as_str()).unwrap_or("").to_string(),
            title: f.get("title").and_then(|s| s.as_str()).unwrap_or("").to_string(),
            families: strs(f.get("families")),
            spec: f.get("spec").and_then(|s| s.as_str()).map(String::from),
        })
        .collect()
}

/// Rule families that appear as `**SPEC-FAMILY-nnn**` in the specification chapters.
pub fn spec_families(ctx: &Ctx) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for f in util::walk(&ctx.root.join("docs/spec")) {
        if !f.ends_with(".md") {
            continue;
        }
        let text = ctx.read(&format!("docs/spec/{f}"));
        let mut rest = text.as_str();
        while let Some(i) = rest.find("**SPEC-") {
            rest = &rest[i + 7..];
            let fam: String = rest.chars().take_while(|c| c.is_ascii_uppercase()).collect();
            let after = &rest[fam.len()..];
            let digits = after.strip_prefix('-').map(|a| a.chars().take_while(|c| c.is_ascii_digit()).count()).unwrap_or(0);
            if !fam.is_empty() && digits >= 3 {
                out.insert(fam);
            }
        }
    }
    out
}

pub fn covered_ids(ctx: &Ctx) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (_, text) in book::pages(ctx) {
        let mut rest = text.as_str();
        while let Some(i) = rest.find("<!-- feature:") {
            rest = &rest[i + 13..];
            if let Some(end) = rest.find("-->") {
                out.insert(rest[..end].trim().to_string());
            }
        }
    }
    out
}

pub fn uncovered(ctx: &Ctx) -> Vec<String> {
    let covered = covered_ids(ctx);
    load(ctx).into_iter().filter(|f| !covered.contains(&f.id)).map(|f| f.id).collect()
}

pub fn check(ctx: &Ctx, cfg: &Config) -> Vec<Finding> {
    let mut out = Vec::new();
    // A book without a registry (a scratch repo in the gate's own tests) has nothing to check; the real book's registry is
    // required by `the_real_registry_lists_every_spec_family`.
    if !util::exists(&ctx.root, REGISTRY) {
        return out;
    }
    let feats = load(ctx);
    if feats.is_empty() {
        return vec![Finding::fail("book", REGISTRY, 0, "the feature registry is empty")];
    }
    let mut ids = BTreeSet::new();
    let mut listed = BTreeSet::new();
    for f in &feats {
        if f.id.is_empty() || f.title.is_empty() {
            out.push(Finding::fail("book", REGISTRY, 0, format!("feature {:?} needs an id and a title", f.id)));
        }
        if !ids.insert(f.id.clone()) {
            out.push(Finding::fail("book", REGISTRY, 0, format!("duplicate feature id `{}`", f.id)));
        }
        if let Some(spec) = &f.spec {
            if !util::exists(&ctx.root, spec) {
                out.push(Finding::fail("book", REGISTRY, 0, format!("feature `{}`: spec chapter {spec} does not exist", f.id)));
            }
        }
        listed.extend(f.families.iter().cloned());
    }
    for fam in spec_families(ctx) {
        if !listed.contains(&fam) {
            out.push(Finding::fail("book", REGISTRY, 0, format!("spec rule family SPEC-{fam} is in no registry entry (an unlisted language feature): add it to a feature in {REGISTRY}")));
        }
    }
    for fam in &listed {
        if !spec_families(ctx).contains(fam) {
            out.push(Finding::fail("book", REGISTRY, 0, format!("registry names family {fam}, which no spec chapter defines")));
        }
    }
    let covered = covered_ids(ctx);
    for id in &covered {
        if !ids.contains(id) {
            out.push(Finding::fail("book", "docs/book/src", 0, format!("a page carries `<!-- feature: {id} -->`, which is not in the registry")));
        }
    }
    let n = feats.iter().filter(|f| !covered.contains(&f.id)).count();
    let max = cfg.book.max_uncovered_features;
    if n > max {
        out.push(Finding::fail("book", "docs/docs-truth.toml", 0, format!("features no page covers: {n} exceeds the ratchet {max} (add `<!-- feature: ID -->` to the page that teaches it); the ratchet only shrinks")));
    } else if n < max {
        out.push(Finding::fail("book", "docs/docs-truth.toml", 0, format!("features no page covers: {n} is below the ratchet {max}: lower `max_uncovered_features` to {n}")));
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn spec_rule_ids_need_three_digits() {
        // `**SPEC-AREA-NNN**` (a placeholder in prose) is not a rule family.
        let t = "**SPEC-PIPE-001** x **SPEC-AREA-NNN** y";
        let mut fams = vec![];
        let mut rest = t;
        while let Some(i) = rest.find("**SPEC-") {
            rest = &rest[i + 7..];
            let fam: String = rest.chars().take_while(|c| c.is_ascii_uppercase()).collect();
            let digits = rest[fam.len()..].strip_prefix('-').map(|a| a.chars().take_while(|c| c.is_ascii_digit()).count()).unwrap_or(0);
            if digits >= 3 {
                fams.push(fam);
            }
        }
        assert_eq!(fams, vec!["PIPE".to_string()]);
    }
}
