# Optimizer passes: preconditions and soundness arguments

Scope: `src/optimizer/` rewrites the Rust text emitted by the code generator.
A pass is ON by default only if this file states sound preconditions, names where
each fact comes from, and the test-suite contains a mutant of the pass that it
catches (`tests/optimizer_mutation_test.rs`). Everything here is about preserving
observable behaviour (stdout, exit status, panics) of the emitted program.

Common machinery
- Parsing: `syn::parse_file`; unparsable input, or a file declaring `macro_rules!`
  (which can bind names the fact collector cannot see), is returned unchanged.
- Edits are byte-range replacements taken from token spans; an edit set whose
  result no longer parses is discarded.
- Fact source (`facts.rs`): a name has a *known type* iff EVERY `Pat::Ident`
  binding of that name inside the enclosing function (parameters, `let`, `if let`,
  `match` arms, `for` patterns, closure parameters, nested items) is a declaration
  with a syntactically evident type, and all those types are equal. Evident types:
  a type annotation (`x: usize`, `String`), a suffixed literal (`3_i32`), a cast
  (`e as f64`), `String::new()` / `String::from(..)`. Any other binding of the same
  name makes the name unknown. Shadowing therefore only ever removes facts.

## P1 temp_clone (ON)
Rewrite: `E.clone()` -> `E` where `E` is `vec![..]`, `String::new()`, `Vec::new()`,
`String::from("lit")` or `"lit".to_string()`.
Why sound: `E` is an rvalue with no other owner; cloning it yields a value equal to
it and then drops the original. Dropping the clone keeps the original value with the
same type (`Clone::clone(&T) -> T`). Element expressions of `vec![..]` are evaluated
exactly once either way. No `Drop` side effects: the dropped temporary is a std
container of the same elements.
Relies on: syntactic shape only.
Caveat: `vec![x]` where `x: !Clone` fails to compile before and after.
Mutant caught: `TempCloneOnAnyIdent` (clone of a live variable, then use of the original).

## P2 copy_clone (ON)
Rewrite: `x.clone()` -> `x` where `x` is a plain identifier with a unique known
primitive type (`i8..u128, isize, usize, f32, f64, bool, char`).
Why sound: for a `Copy` primitive, `clone()` is the identity and `x` is not moved by
use. References (`&i32`) are never "known primitive" (the annotation is `&i32`, not
`i32`), so `r.clone()` (which would yield `i32`, not `&i32`) is untouched.
Relies on: uniqueness rule above (shadowing by a String/pattern binding disables it).
Mutant caught: `CopyCloneIgnoresShadowing` (`let n = s; n.clone()` after `n: usize`).

## P3 same_type_cast (ON)
Rewrite: `(e as T) as T` -> `e as T`; `x as T` -> `x` where `x` has unique known type `T`.
Why sound: `as T` on a value already of type `T` is the identity (all primitive
casts), including float-to-float and int-to-int of equal type. The nested form is the
identity on the outer cast because the inner cast already produced a `T`.
Mutant caught: `CastDropsAnyIdent` (drops a narrowing cast such as `x as u8`).

## P4 with_capacity (ON)
Rewrite: `let mut v = Vec::new(); for _ in 0_usize..N { ...; v.push(..); ... }` ->
`Vec::with_capacity(N)`.
Preconditions (all checked): the `for` is the statement right after the `let`;
range start is the literal `0_usize` and exclusive; `N` is built only from
identifiers, integer literals, field paths, `+ - *`, casts, and `.len()` on a place
(no calls, no side effects); the loop body has a top-level unconditional
`v.push(..);`; the body contains no `break`, `continue`, `return`, `?` (also inside
macro token streams); the file never calls `.capacity()`.
Why sound: capacity is not part of `Vec`'s observable semantics except via
`.capacity()` (excluded). `N` is evaluated once more, immediately before the loop
head evaluates it again, with no statement in between, so panics (overflow) occur
identically and in the same order. With no early exit, the loop pushes at least
once per iteration, so the reservation is never larger than the final length by
more than what the program would allocate anyway; with `break` it could be absurdly
larger (capacity overflow), hence the exclusion.
Mutants caught: `CapacityIgnoresBreak`, `CapacityAnyStart`.

## P5 string_append (ON)
Rewrite: `s = format!("{}<rest>", s, args..);` ->
`{ use std::fmt::Write as _; write!(s, "<rest>", args..).expect(..); }`.
Preconditions: `s` has unique known type `String`; the literal is non-raw and begins
with an unadorned `{}` consumed by the first argument (the identifier `s`); the
remainder uses only positional placeholders (no `{0}`, `{name}`, `$`, `*`); no other
argument mentions `s`.
Why sound: `format!("{}R", s, a..)` evaluates `a..` in order then builds `s + R(a..)`;
`write!(s, "R", a..)` evaluates `a..` in the same order and appends `R(a..)` to `s`.
Because no argument mentions `s`, there is no aliasing of the mutable borrow and no
argument can observe the in-place state. Both are linear in the appended part, the
original is O(len(s)) per statement.
Known deviation (documented, accepted): if a `Display` impl PANICS mid-write and the
panic is caught (`catch_unwind`) and `s` is read afterwards, `s` holds a partial
append whereas the original left it unchanged. `Display` returning `Err` panics in
`format!` and, via `.expect`, here as well.
Mutant caught: `AppendIgnoresSelfReference` (`u = format!("{}{}", u, u)` ->
borrow-check failure).
Note: the type check is belt-and-braces: assigning `format!` to `s` already forces
`s: String`, so a mutant that skips it is behaviourally equivalent and not listed.

## P6 inline_policy (ON)
Rewrite: delete `#[inline]` on `#[test]`/`#[bench]` fns, `main`, and fns whose body
exceeds `inline_max_body_bytes` (600).
Why sound: `#[inline]` is a hint; it has no semantic effect. Benefit: less code
growth and faster compiles. No claim of run-time speedup is made.

## P7 dead_helpers (ON)
Rewrite: (a) remove `#[repr(C)]` on non-`pub` structs; (b) remove an inherent impl
containing only `to_bytes` for a non-`pub` struct when the identifier `to_bytes`
occurs nowhere else in the file.
Why sound: a non-`pub` item cannot be named from another module, so the whole file
is the complete set of users. (a) requires that the file contains none of
`unsafe, extern, transmute, size_of, align_of, offset_of, as_ptr, as_mut_ptr,
from_raw_parts, MaybeUninit, addr_of, bytemuck, Pod, Zeroable`: without them the
program cannot observe field order or padding. Dropping `repr(C)` lets rustc reorder
fields (smaller structs). (b) an unreferenced private method has no callers
(trait-object and macro paths are covered by the token scan of the entire file).
Mutants caught: `ReprIgnoresUnsafe` (size_of::<S>() changes), `HelpersIgnoreReferences`.
Residual risk: the code generator must never expect a private struct from file A to
be reachable from file B. Rust itself enforces this (privacy error), so a violation
would already fail to compile.

## Considered and rejected (stay OFF / not implemented)
- Last-use clone elimination (`let y = x.clone();` with `x` unused afterwards): needs
  borrow facts (a live `&x` makes the move a compile error) and drop-order facts
  (`impl Drop`); neither is available from a text pass. The code generator's
  `auto_clone` analysis is the right place; we do not duplicate it.
- `to_string()`-of-literal avoidance: depends on callee parameter types, which are
  unknown to a text pass; belongs in codegen with the signature registry.
- Index loops -> iterators: changes bounds-check behaviour under panics and
  aliasing; no benefit measured (LLVM already removes the checks in the kernels).
- SIMD, escape analysis, string interning, AST loop invariants (retired phases 11-15):
  see ASSESSMENT.md.
