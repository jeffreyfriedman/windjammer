# portable (shared Go / JS / TS backend front-end)

## Purpose
Turns a parsed Windjammer `Program` into a typed, statement-oriented IR (`PModule`) that the Go and
JavaScript/TypeScript emitters print. All semantics that must agree across backends live here, not in the emitters.


## Inputs/Outputs
In: parsed `Program` plus the analyzer `SignatureRegistry`. Out: `PModule`.
Pipeline: `parser AST -> lower (typing, ownership, desugaring) -> flatten -> emitter`.


| file | role |
|------|------|
| `ty.rs` | `Ty` / `IntKind`: exact integer widths (the parser collapses `i8..u64` into `Custom`) |
| `alias.rs` | `type` aliases: installed per `lower_program`, expanded by `ty::from_ast`; `Alias { .. }` / `Alias::f()` resolve to the nominal type |
| `env.rs` | item tables (structs, enums, fns, methods, traits) + parameter modes from the analyzer registry |
| `ir.rs` | `PModule`, `PFn`, `PStmt`, `PExpr`/`PK`; `Unsupported` (explicit "this construct is not implemented") |
| `lower/` | bidirectional type inference with inference variables, `?`, interpolation, patterns, value-semantics `Copy` insertion |
| `lower/intrinsics.rs` | ADR-0014 intrinsics (`print`/`assert*`/`panic`/`todo`/`unreachable`/`debug`/list literal) desugared to `Print`/`Panic`/`If`/`Block`; names come only from `crate::intrinsics` |
| `lower/fmt.rs` | `{:spec}` parsing (Rust's full flag set: fill/align, `+`, `#`, `0`, width, precision); unsupported types (`E`, `x?`, `#e`) are explicit errors |
| `builtins.rs` | typing rules of std methods (`Vec`, `String`, `HashMap`, `Option`, iterators, ints, floats) |
| `flatten.rs` | expression-position `if`/`match`/block/`?` become statements, preserving evaluation order |
| `visit.rs` | mutable traversals (type resolution after inference) |


## Invariants
- After `lower_program` + `flatten_module`, every `PExpr.ty` is resolved (or an emitter reports "type could not be inferred"),
  expressions are pure trees, and `PBlock.tail` is `None`.
- Value semantics: an owned use of a place of aggregate type is wrapped in `PK::Copy`; `&`/`&mut` parameter modes
  come from `analyzer::SignatureRegistry` (`ParamMode::{Value, Ref, MutRef}`), so emitters never guess ownership.
- Anything not implemented returns `Unsupported` (compile error) - never placeholder code.
- `let r = &mut place` lowers to `Let { init: PK::BorrowMut(place) }` and `r` is an alias local: emitters
  bind a pointer (Go) or the shared object / an accessor cell (JS/TS), never a copy. Other `&`/`&mut`/`*` are
  erased; `&mut` arguments are recognised by the callee's `ParamMode::MutRef`.


## Owns
Typing and ownership-driven copies.

## Never decides
Syntax, runtime representation of integers/strings, or naming (emitters do).


## Allowed imports
`crate::intrinsics` (the intrinsic name registry; never match `println`/`panic`/... as strings here), `crate::parser`, `crate::analyzer` (registry only), `crate::ir::analyze_and_lower` (done by backends).


## Entry points
`lower::lower_program(&Program, Some(&registry))`, `flatten::flatten_module(&mut PModule)`.


## Test
Unit tests next to each module; end-to-end through `tests/backend_conformance_test.rs`
(runs generated Go/JS against independent oracles). Add a corpus program in `tests/conformance_programs/` for every feature.

## Closure-literal callee
`(|v| body)(x)` (the desugaring of `x |> |v| body`) lowers the arguments first and types the
closure parameters from them (`lower/call.rs`), so Go/JS agree with the argument's width.
Corpus: `tests/conformance_programs/150_pipe_forms_mixed.wj`, `151_pipe_try.wj`.
