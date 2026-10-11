# portable (shared Go / JS / TS backend front-end)

**Purpose.** Turns a parsed Windjammer `Program` into a typed, statement-oriented IR (`PModule`) that the Go and
JavaScript/TypeScript emitters print. All semantics that must agree across backends live here, not in the emitters.

**Pipeline.** `parser AST -> lower (typing, ownership, desugaring) -> flatten -> emitter`.

| file | role |
|------|------|
| `ty.rs` | `Ty` / `IntKind`: exact integer widths (the parser collapses `i8..u64` into `Custom`) |
| `env.rs` | item tables (structs, enums, fns, methods, traits) + parameter modes from the analyzer registry |
| `ir.rs` | `PModule`, `PFn`, `PStmt`, `PExpr`/`PK`; `Unsupported` (explicit "this construct is not implemented") |
| `lower/` | bidirectional type inference with inference variables, `?`, interpolation, patterns, value-semantics `Copy` insertion |
| `builtins.rs` | typing rules of std methods (`Vec`, `String`, `HashMap`, `Option`, iterators, ints, floats) |
| `flatten.rs` | expression-position `if`/`match`/block/`?` become statements, preserving evaluation order |
| `visit.rs` | mutable traversals (type resolution after inference) |

**Invariants.**
- After `lower_program` + `flatten_module`, every `PExpr.ty` is resolved (or an emitter reports "type could not be inferred"),
  expressions are pure trees, and `PBlock.tail` is `None`.
- Value semantics: an owned use of a place of aggregate type is wrapped in `PK::Copy`; `&`/`&mut` parameter modes
  come from `analyzer::SignatureRegistry` (`ParamMode::{Value, Ref, MutRef}`), so emitters never guess ownership.
- Anything not implemented returns `Unsupported` (compile error) - never placeholder code.

**Owns / never decides.** Owns typing and ownership-driven copies. Never decides syntax, runtime representation of
integers/strings, or naming (emitters do).

**Allowed imports.** `crate::parser`, `crate::analyzer` (registry only), `crate::ir::analyze_and_lower` (done by backends).

**Entry points.** `lower::lower_program(&Program, Some(&registry))`, `flatten::flatten_module(&mut PModule)`.

**Testing.** Unit tests next to each module; end-to-end through `tests/backend_conformance_test.rs`
(runs generated Go/JS against independent oracles). Add a corpus program in `tests/conformance_programs/` for every feature.
