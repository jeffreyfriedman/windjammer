---
status: current
last_verified: 7b2696853fd6
---
# Visibility in emitted Rust

Rule: an item is `pub` in the Rust output only when the Windjammer source writes `pub`.

| Item | Emitted | Where |
|---|---|---|
| free `fn` | `pub fn` only when written `pub` (a private helper stays private, also in library modules) | `function_generation_signature.rs` |
| `struct` / `enum` / `const` | as written | item generation |
| struct fields | as written | data structure generation |
| inherent methods | `pub` only when written `pub` | impl generation |
| `trait` | always `pub trait` | `item_generation.rs`, `generate_trait_with_analysis` (open TODO) |

Traits are the one exception, and it is a recorded shortcut, not a design decision: `TraitDecl` has no `is_pub`
field yet, and a private trait named in a `pub fn` signature would be a Rust error, so the emitter picks the safe
side. Making it exact means adding `is_pub` to the parser, then emitting `pub trait` only when written (and
checking that no public item names a private trait). Until then a private trait is visible to other modules of the
crate and to dependents; it cannot cause a compile error.

Gate: `private_fn_not_glob_exported` (project corpus) pins the free-function rule.
