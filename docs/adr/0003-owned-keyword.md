# ADR-0003: The `owned` keyword

- **Status:** Proposed
- **Date:** 2026-10-10
- **Related:** [WJ-LANG-02](../proposals/rfcs/WJ-LANG-02-mojo-feature-analysis.md), `src/lexer/identifier_lexing.rs`

## Context
Mojo-style argument conventions (`owned`, `borrowed`, `inout`) appear in WJ-LANG-02 as a candidate. There is no
`owned` token in the lexer (`grep` of `src/lexer` finds none); the only hits for "owned" in the compiler are
internal ownership-mode names. Ownership is inferred from the function body (ADR-0001). The open question is whether
users ever need to force a mode, for example to make an API take ownership for an FFI boundary, a long-lived store,
or to stop the solver from choosing a borrow that later breaks a caller.

## Options
### A. No keyword; inference only
- Pros: consistent with "no Rust leakage"; no new syntax to document or keep in four backends.
- Cons: no escape hatch when inference picks the wrong mode; API stability depends on body details.

### B. `owned` parameter modifier (explicit override)
- Pros: stable public signatures; an FFI-friendly escape hatch.
- Cons: second way to say the same thing; users start annotating everything; solver must treat as a hard constraint.

### C. Signature-level attribute (`@owned` on a parameter) used only on `pub` API
A narrower B: allowed only where a signature is a public contract.

## Recommendation
Option A now. Revisit with C only after ADR-0001 is implemented and there is a concrete case (public API stability or
extern boundary, ADR-0012) that inference cannot express. No parser work before then.

## Consequences
- WJ-LANG-02 `owned` section stays "Not started"; the RFC table says so.
- Users who need owned semantics rely on `.clone()`/move patterns the solver already handles.

## Decision needed from: project owner
Confirm A (reject for now), or authorise C for `pub` API signatures.
