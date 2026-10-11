//! Proposal documents: design text for features that do not exist yet.
//!
//! A document whose front matter says `status: proposal` describes the future. The checkers treat it
//! accordingly: `wj` commands/flags, error codes and Windjammer fences in it are *future* and are not
//! checked against the compiler, but every proposal must carry a visible banner so no reader mistakes it
//! for current behavior, and links/paths to things that do exist are still verified.
//!
//! Everything under `docs/proposals/` must declare a status (`proposal`, `rejected`, `superseded`,
//! `archived`, `accepted` or `current`), so a doc cannot dodge the checks by omitting front matter.

use super::{freshness, util, Ctx, Finding};

pub const BANNER_PREFIX: &str = "> **Proposal";
/// The banner must appear within this many lines after the front matter.
pub const BANNER_WINDOW: usize = 12;
pub const REQUIRE_STATUS_GLOB: &str = "docs/proposals/**/*.md";

pub fn is_proposal(text: &str) -> bool {
    freshness::front_matter(text).and_then(|m| m.status).is_some_and(|s| s.eq_ignore_ascii_case("proposal"))
}

/// Banner check for one document (empty for non-proposals).
pub fn check_doc(file: &str, text: &str) -> Vec<Finding> {
    if !is_proposal(text) {
        return vec![];
    }
    let body = text.strip_prefix("---\n").and_then(|r| r.split_once("\n---\n")).map(|x| x.1).unwrap_or("");
    let ok = body.lines().take(BANNER_WINDOW + 1).any(|l| l.starts_with(BANNER_PREFIX) && l.to_lowercase().contains("not implemented"));
    if ok {
        vec![]
    } else {
        vec![Finding::fail("proposal", file, 1, format!("`status: proposal` documents need a banner within {BANNER_WINDOW} lines of the front matter: `{BANNER_PREFIX}: not implemented.** ...`"))]
    }
}

pub fn check_all(ctx: &Ctx) -> Vec<Finding> {
    let mut out = Vec::new();
    for f in util::walk(&ctx.root).into_iter().filter(|f| f.ends_with(".md")) {
        let text = ctx.read(&f);
        out.extend(check_doc(&f, &text));
        if util::glob_match(REQUIRE_STATUS_GLOB, &f) && !f.ends_with("/README.md") && freshness::front_matter(&text).and_then(|m| m.status).is_none() {
            out.push(Finding::fail("proposal", &f, 1, "documents under docs/proposals/ must declare front matter `status:` (proposal, rejected, superseded, archived, accepted or current)"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn banner_required_only_for_proposals() {
        assert!(check_doc("a.md", "# T\n").is_empty());
        assert!(check_doc("a.md", "---\nstatus: current\n---\n# T\n").is_empty());
        assert_eq!(check_doc("a.md", "---\nstatus: proposal\n---\n# T\n").len(), 1);
        assert!(check_doc("a.md", "---\nstatus: proposal\n---\n# T\n\n> **Proposal: not implemented.** Future.\n").is_empty());
        assert_eq!(check_doc("a.md", "---\nstatus: proposal\n---\n# T\n\n> **Proposal.** hi\n").len(), 1);
    }
}
