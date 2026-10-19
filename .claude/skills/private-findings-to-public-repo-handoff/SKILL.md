---
name: private-findings-to-public-repo-handoff
description: Use when porting private findings into the public repo.
category: dogfood-triage
---

# Private findings to public repo handoff

The public compiler repo must never carry downstream project names, paths, or ticket ids. The subagent brief is the enforcement point, so write it that way.

- Put a no-private-names preamble first in every brief that touches the public repo, and have the subagent read it before any other step.
- Mark the private source read-only and say the subagent may use it for reference only, copying failing behavior and never names, paths, or identifiers.
- Name every test by behavior (`bug_<behavior>_test.rs`), never by the source project.
- Hand over the reduced generic repro, not the full downstream file, whenever one exists.
- Ask for per-class reporting: ported, failing, fixed, handed off.
- When a class belongs to another stream, hand it off with its acceptance test and do not re-decide it on the new branch.

Reference: [references/brief-template.md](references/brief-template.md) has the brief skeleton.
