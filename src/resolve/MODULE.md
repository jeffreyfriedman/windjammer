# resolve

Identity-based name resolution, run in **shadow mode** for diagnostics; the interpreter loader binds names with it, and codegen asks it one question today: `same_crate_callee_key` (is this qualified callee a same-crate definition, through `use` aliases, `self::`, `super::`).

## Purpose
Every definition gets a `DefId`; every call expression resolves to a `Res`: `Def(DefId)`, `Local`,
`Prim`, `Unresolved(Reason)`, `Ambiguous(Vec<DefId>)` or (method calls on unknown receivers)
`MethodCandidates(Vec<DefId>)`. Ambiguity is an error, never a collision set.

## Inputs / outputs
- In: `&[ParsedFile]` (module path + parser `Program`). Pure; no IO, no globals.
- Out: `ResolvedProgram { collected (DefTable + scopes), calls: Vec<CallResolution>, report }`.
- `ResolveReport`: counts (resolved / method_candidates / unresolved / ambiguous, by reason) + diagnostics.

## Pipeline (files)
1. `collect.rs` - allocate defs, module scopes, impls, bodies.
2. `lookup.rs`  - scope lookup (items, `use`, alias, glob, `pub use`, `crate/self/super`), impl linking.
3. `paths.rs`   - callee paths: `f`, `mod::f`, `Type::assoc`, `Self::assoc`, `Trait::m`, `Enum::V`.
4. `walk.rs`    - walks bodies, tracks locals (shadowing), emits `CallResolution`s.
`def.rs` / `res.rs` / `uses.rs` / `prelude.rs` are data + normalisation.

## Invariants
- A free name resolves through scope only; methods live in impls and never compete with free fns.
- Inherent assoc items beat trait items; two trait impls providing the same name are `Ambiguous`.
- Unknown-receiver method calls are filtered by name and arity only (no name heuristics beyond that).
- `Unresolved(External)` = provided outside the program (std/runtime/other crate); `NotFound` is a bug or user error.

## Owns / never decides
Owns: identity and lexical resolution. Never decides: types of expressions, ownership, codegen shape.

## Allowed imports
`crate::parser::ast`, `crate::source_map`. Not `analyzer`, `codegen`, `compiler`.

## Entry points
`resolve_program`, `module_path_from_file`; driver hook: `compiler::resolve_project_shadow`.

## Testing
`cargo test --release --test all resolve_` (core, imports, fixtures, shadow). Shadow diffs:
`docs/resolve-shadow-diffs.md` (regenerate via the ignored `write_shadow_diffs_doc` test).
