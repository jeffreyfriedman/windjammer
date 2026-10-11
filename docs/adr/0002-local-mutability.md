---
status: accepted
last_verified: 3406ef05bb35
---
# ADR-0002: Local mutability: inferred vs explicit

- **Status:** Accepted (2026-10-10) — explicit `let mut` (Rust-like). Rationale (owner): inferred mutability can cause surprising, unintended behavior. Ownership of references (&, &mut, owned) stays inferred; only local binding mutability is explicit. Migration: compiler diagnostic with an automatic fix (`wj fix`) that adds `mut`, then remove `auto_needs_mut`.
- **Date:** 2026-10-10
- **Related:** [language-design/ownership-inference.md](../language-design/ownership-inference.md), `src/codegen/rust/let_statement_generation.rs`, `src/codegen/rust/function_formal_parameter_generation.rs`, `src/codegen/rust/variable_analysis/mut_inference.rs`

## Context
The language design doc states that Windjammer infers borrow modes but "keeps mutability (`let mut`) explicit"
and that "the compiler does not convert `let` to `let mut`". The code does the opposite in practice:
`let_statement_generation.rs` computes `auto_needs_mut = !mutable && ... variable_needs_mut(name)` and emits
`let mut` when the body mutates the binding; `function_formal_parameter_generation.rs` does the same for
parameters (`auto_needs_mut`); `variable_analysis/mut_inference.rs` implements the inference. The grammar
(`docs/windjammer.ebnf`) still has `[ "mut" ]` on `let`, so both spellings compile. Users and docs cannot tell
which is canonical, and a mutation-through-alias bug in either direction changes program meaning.

## Options
### A. Infer local mutability; `mut` is optional and linted
Matches "the compiler does the hard work" and the 80/20 philosophy. `mut` becomes redundant; add a formatter/lint rule.
- Cons: mutation is no longer visible at the declaration; the old doc's "accidental mutation" concern applies.

### B. Explicit `let mut` required; inference removed (error instead)
Keeps a visible safety guardrail. Cons: contradicts current behaviour; breaks existing `.wj` sources that rely on inference; more user-visible Rust-isms.

### C. Hybrid (current behaviour, undocumented)
Infer silently, accept explicit `mut`. Cons: no canonical style; the two diverge when inference has bugs.

## Recommendation
Option A, with a compiler warning-level lint (`W-mut-redundant`) off by default and the formatter normalising to
"no `mut` on locals". Parameters and `self` are already inferred and stay so. Rewrite ownership-inference.md to match.

If the owner prefers the explicitness of B, the fix is a diagnostic ("`x` is mutated but not declared `mut`") instead of silent insertion; either way the doc and code must agree.

## Consequences
- ownership-inference.md, the language guide and examples change; EBNF keeps `[ "mut" ]` for compatibility.
- Tests that pin `let mut` output remain valid (explicit `mut` still honoured).
- `variable_analysis/mut_inference.rs` becomes a documented, tested contract rather than a convenience.

## Decision needed from: project owner
Choose A, B or C. This is a user-visible language decision.
