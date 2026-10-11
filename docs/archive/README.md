# Archive

Historical documents kept for context only. They describe designs, status snapshots or audits that are
stale or superseded; do not treat them as current. For the live state see
[ARCHITECTURE.md](../ARCHITECTURE.md), [known-gaps.md](../known-gaps.md) and [adr/](../adr/README.md).

| Path | Why archived |
|------|--------------|
| `design/auto-reference.md` | Pre-solver design for automatic `&` insertion; superseded by the IR solver (ADR-0001). |
| `language-design/WGSL_TRANSPILER_DESIGN.md` | Early WGSL design; the shipped front end is WJSL (RFC WJ-LANG-01). |
| `language-design/deref-logic.md` | Point-in-time design note for string comparison deref; behaviour now lives in codegen/IR coercion. |
| `language-design/consistency-audit.md` | Nov 2025 audit snapshot. |
| `PLATFORM_ABSTRACTION.md` | Aspirational platform layer; not implemented (see ADR-0007, ADR-0008). |
| `implementation/*` | Dated status/bug-list snapshots (test status, compiler fix lists, WGSL transpiler status). |
