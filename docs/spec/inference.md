# Windjammer inference specification

Status: draft 1 (2026-10-11). Normative for ownership, mutability, clone and numeric-width inference.
Sources: ADR-0001 (the IR solver is the sole owner of decisions), ADR-0002 (local mutability is explicit),
ADR-0015 (one default numeric type), `src/ir/solver.rs`, `src/ir/constraint_gen.rs`, `src/analyzer/`.

How to read this document:

* Every rule has a stable ID. The IDs are the same ones `wj explain-ownership` prints and the same ones
  `src/ir/explain/rules.rs` lists (a test keeps the three in sync).
* **Status** says whether the compiler follows the rule today. `Conforms` was checked against emitted Rust;
  `Diverges (D-...)` means the implementation disagrees with the rule or with itself, and the repro is in
  [inference-divergences.md](inference-divergences.md); `Implementation-defined` means the behavior exists but no ADR
  decides it yet (it is documented here so it can be reviewed, not because it is endorsed).
* Examples are Windjammer source plus the signature the compiler emits, observed on this tree.

## 0. Model

A Windjammer program has value semantics: assigning or passing a value never lets the callee observe later
changes through another name, except through an explicit `&mut` parameter. The compiler picks the cheapest Rust
realisation that preserves that meaning:

1. each **parameter** gets a mode: shared borrow `&T`, exclusive borrow `&mut T`, or owned `T`;
2. each **use of a value** is a copy (Copy types), a move (last use), or a clone (value used again later);
3. each **numeric value** gets a width.

The user writes none of these. Two surface controls exist: explicit `&T` / `&mut T` parameter annotations (a
*pin*, section 8) and explicit numeric annotations or literal suffixes.

Notation: modes are ordered `&` < `&mut` < `owned` (the lattice of OWN-LATTICE).

## 1. Parameter modes

### OWN-LATTICE
**Rule.** A parameter's mode is the join of the requirements every use places on it, in the order
`&` < `&mut` < `owned`. A use never lowers the requirement of another use.
**Why.** An owned requirement (the value is returned or stored) cannot be met by a borrow; a mutation cannot be met
by a shared borrow. Taking the maximum is the smallest mode that makes every use valid, and it is order-independent.
**Example.** `fn f(v: Vec<int>) -> Vec<int> { v.push(1); v }` pushes (`&mut`) and returns (`owned`): emitted `mut v: Vec<i64>`.
**Status.** Conforms in `src/ir/solver.rs` (`Solver::process_constraint`, `OwnershipIs`), but see D-SOLVER-UNION:
constraints that arrive through a `TypeEquals` union are lost, and D-OWNERS-OWN: the final mode is decided by four
components, not by the join.

### OWN-READONLY
**Rule.** A non-Copy parameter that is only read (field reads, method calls that take `&self`, comparisons,
interpolation, `len()`, iteration that does not consume it) is `&T`.
**Why.** Borrowing is free and lets callers keep using the argument without a clone.
**Example.** `fn n(b: Bag) -> int { b.items.len() }` emits `fn n(b: &Bag)`.
**Status.** Diverges (D-OWN-FIELD-RETURN-COPY, D-OWN-OPTION-MATCH, D-CLOS-OVEROWN): a few read-only shapes are
emitted owned, which forces callers to clone.

### OWN-MUT-ASSIGN
**Rule.** Assigning to a parameter, to one of its fields, or to an element of it makes it `&mut T`.
**Why.** The caller must observe the mutation (value semantics for the caller's variable are preserved because the
caller passes `&mut` of its own binding).
**Example.** `fn bump(c: Cell) { c.n = c.n + 1 }` emits `fn bump(c: &mut Cell)`; the caller writes `bump(c)` and the
compiler emits `bump(&mut c)`.
**Status.** Conforms.

### OWN-MUT-RECEIVER
**Rule.** Calling a method whose receiver is `&mut self` on a parameter (or on a field path rooted at it) makes the
parameter `&mut T`. Whether the method mutates is itself inferred by SELF-MUT, so the rule is transitive over the call
graph and is iterated to a fixed point across functions.
**Example.** `fn add(b: Bag, s: string) { b.items.push(s) }` emits `b: &mut Bag, s: String`.
**Status.** Conforms for same-file callees; cross-file needs the converged registry (D-OWNERS-OWN, ratchet residue 4).

### OWN-MUT-ANALYZER
Implementation seed: the analyzer's `mutated_parameters` set is replayed as a `&mut` constraint. Not a language
rule; it exists because the solver's own body walk is incomplete. Status: Diverges (D-OWNERS-OWN).

### OWN-RETURNED
**Rule.** A parameter that is returned, or a non-Copy value moved out of one of its fields and returned, is owned.
**Why.** The result is an owned value (RET-OWNED); producing it from a borrow would need a clone.
**Example.** `fn keep(v: Vec<int>) -> Vec<int> { v }` emits `fn keep(v: Vec<i64>)`.
**Status.** Conforms for whole-parameter returns. For `self` the implementation chooses differently, see SELF-GETTER.

### OWN-STORED
**Rule.** A parameter moved into a struct literal, enum payload, tuple, array, `Some(..)`, or a collection method that
stores its argument (`push`, `insert`) is owned.
**Example.** `fn wrap(b: Bag, t: string) -> Holder { Holder { bag: b, tag: t } }` emits both parameters owned.
**Status.** Diverges in one shape (D-OWN-STORE-ASSIGN): storing through a field assignment, `h.tag = n`, demotes `n`
to `&str` and copies inside, while `push(n)` keeps `n` owned and moves.

### OWN-COPY-SEED
**Rule.** Parameters of Copy type (numbers, `bool`, `char`, structs and enums all of whose fields are Copy) are passed
by value unless the body mutates them (then `&mut`).
**Example.** `struct P { x: int, y: int }` is auto-derived Copy; `fn sum(p: P) -> int { p.x + p.y }` emits `p: P`.
**Status.** Conforms.

### OWN-READ-FIELD
**Rule.** A field projection that is only read borrows the root parameter (`&`), even for a non-Copy field, as long as
the value read is Copy or is cloned when stored (SELF-GETTER / CLONE-BORROWED-TO-OWNED).
**Status.** Conforms except D-OWN-FIELD-RETURN-COPY (a bare `i.qty` returned from a non-Copy parameter makes the
whole parameter owned).

### OWN-CALL-FORMAL
**Rule.** A parameter passed on, by name, to a callee inherits the callee's formal mode, with the same join.
`wrapper(b) { process(b) }` is `&` when `process` borrows. The rule is cyclic for mutual recursion; the fixed point
starts from `&` (least requirement) and only rises.
**Status.** Diverges: the constraint is generated (`emit_call_site_constraints`) but never reaches the parameter in
the solver (D-SOLVER-UNION). The effective behavior comes from the analyzer (`passthrough_inference.rs`).

### OWN-CONSUMING-METHOD
**Rule.** A consuming trait method (`into`, `try_into`, `into_iter`) called on a parameter whose declared type is one of
the function's own type parameters takes the parameter by value: `fn label<T: Into<string>>(t: T) -> string { t.into() }`
makes `t` owned.
**Status.** Conforms (C5 category 5). The analyzer still seeds generic parameters owned (`parameter_analysis.rs`
`is_generic_type_param`); that seed is on the deletion list.

### OWN-STR-REF
**Rule.** A read-only `string` parameter of a free function is emitted `&str`, not `&String`.
**Status.** Implementation-defined for visibility: an unused or interpolation-only `string` formal is `String` when the
function is `pub` and `&str` when private (D-OWN-PUB). This should be a rule about use, not about `pub`.

### OWN-PIN-REF, OWN-PIN-MUTREF
**Rule.** An explicit `&T` / `&mut T` annotation on a parameter fixes the mode as a lower bound: inference may not
choose a weaker mode, and if the body needs more (it returns or stores the parameter) the compiler must report an
error at the annotation, not silently change the mode.
**Why.** A pin is a statement about the public contract; changing it would break callers invisibly.
**Status.** Diverges (D-PIN-DOUBLE-REF, D-PIN-NOT-HONORED). See section 8 and ADR-0016.

### OWN-ANALYZER-SEED
Not a language rule. The analyzer's decision (`AnalyzedFunction::inferred_ownership`) is fed to the solver as a
constraint and re-applied after solving by `finalize_ir_param_ownership_from_analyzer` and six sibling passes.
It is listed so `wj explain-ownership` can name it. ADR-0001 requires it to disappear. Status: Diverges (D-OWNERS-OWN).

## 2. Receivers

### SELF-READ
**Rule.** A method that only reads `self` takes `&self`.
**Example.** `fn get(self) -> int { self.n }` emits `fn get(&self) -> i64`.
**Status.** Conforms.

### SELF-MUT
**Rule.** A method that assigns to a field of `self`, calls a `&mut self` method on `self` or on a field of it, or
mutates through a `self` field that is an `Option` scrutinee, takes `&mut self`.
**Status.** Conforms.

### SELF-CONSUME
**Rule.** A method that returns `self` (builder style) or moves it into a store takes `self` by value.
**Example.** `fn chain(self) -> Counter { self.bump(); self }` emits `fn chain(mut self)`.
**Status.** Conforms (the `mut` is inserted silently, see MUT-LOCAL-EXPLICIT / D-MUT-LOCAL).

### SELF-GETTER
**Rule.** A method that returns a non-Copy field of `self` takes `&self` and clones the field in the result.
**Why (documented in `parameter_analysis.rs`).** Callers keep using the object after calling a getter.
**Example.** `fn name_of(self) -> string { self.name }` emits `fn name_of(&self) -> String { self.name.clone() }`.
**Status.** Diverges from the parameter rule (D-SELF-VS-PARAM): the identical body on a *parameter*,
`fn name_of(b: Bag) -> string { b.name }`, makes the parameter owned and moves the field out. The two cases are
decided by `self_return_and_consumption.rs` and `parameter_analysis.rs` respectively. One rule must win; SELF-GETTER is
the better contract (no clone at the call site), so the parameter case should follow it.

## 3. Moves, copies, clones

### MOVE-COPY-TYPE
**Rule.** A value of Copy type is copied at every use. It is never moved and never cloned.
**Status.** Conforms.

### MOVE-LAST-USE
**Rule.** The last use of an owned non-Copy value moves it. No clone is emitted at the last use.
**Example.** `let b2 = consume(b)` with `b` unused afterwards emits `consume(b)`.
**Decided by.** The liveness fact `ir::liveness::last_moves` (per read, by node identity; parameters and plain
`let` bindings only; a read is not last inside a loop the binding outlives, inside a closure, or while a binding
initialised from it may still hold a borrow). The Rust backend hands the bindings that die in each statement to every reuse-clone
query (`codegen/rust/move_at_last_use.rs`), so no statement-index heuristic can re-add a clone there.
**Status.** Conforms for bare owned bindings (`tests/inference_corpus/move_at_last_use.wj`: `wrap(b, ..)`
after `push_bag(&mut b, ..)`, `self.touch(k)` after `self.order.push(k.clone())`). Mutation check: a
liveness that marks every read last is rejected by rustc (`mutation_always_move_liveness_is_rejected`).
Open: a clone of an element or field (`svo[0].clone()`) is a place clone, not a last-use decision
(CLONE-BORROWED-TO-OWNED); `explain` still reports it as D-CLONE-LASTUSE.

### CLONE-USE-AFTER-MOVE
**Rule.** If a non-Copy value is passed to an owned formal (or moved into a store) and is used again later, a clone is
inserted at the move, never at the later use.
**Example.** `take(it); use(it)` emits `take(it.clone()); use(it)`.
**Decided by.** The liveness fact `ir::liveness::read_after` (per statement: bindings read later, read in the next
iteration of a loop they outlive, or kept alive by a derived binding such as a capturing closure). The Rust backend's
reuse checks (`local_binding_reused_after_current_statement`, `param_has_later_owned_formal_pass`) read it instead of
scanning the current block's later statements (which missed uses after the enclosing block and mixed statement indices).
**Status.** Conforms for the earlier site; see MOVE-LAST-USE for the clone at the final site. Also diverges for values bound to a string literal (D-CLONE-STRLIT): `let t = s` followed by a use of `s`
moves `s` and rustc rejects the program.

### CLONE-LOOP
**Rule.** A value moved inside a loop body whose binding is declared outside the loop is cloned at the move.
**Example.** `for k in 0..3 { take(it3) }` emits `take(it3.clone())`.
**Status.** Conforms.

### CLONE-BORROWED-TO-OWNED
**Rule.** Where an owned value is required and only a borrow is available (returning a non-Copy field of a borrowed
parameter or `self`; passing a borrowed parameter to an owned formal), the value is cloned (`.clone()`, or
`.to_string()` for text).
**Status.** Conforms. Over-clones exist where the clone is applied and then borrowed again: `prefix + &self.name.clone()`
(D-CLONE-REDUNDANT).

### CLONE-ANALYZER
Implementation seed for the solver's `NeedsClone` constraint (analyzer `auto_clone` sites; the never-called
use counting `CLONE-MULTI-USE` was deleted in C6). The solver's
clone result is not consumed by any code (D-CLONE-OWNERS); clones come from `auto_clone.rs` and about two thousand
`.clone()` emission sites in codegen. These IDs exist so `explain` can show the constraint that *would* decide.

## 4. Returns

### RET-OWNED
**Rule.** A function's result is an owned value. Ownership of the result never depends on the body shape.
**Status.** Conforms.

### RET-NO-BORROW
**Rule.** A function does not return a reference into its parameters. Windjammer has no lifetime syntax; a value that
must outlive the call is cloned (CLONE-BORROWED-TO-OWNED).
**Status.** Conforms for inferred signatures. A pinned `&T` parameter returned as owned is emitted unchecked
(D-PIN-NOT-HONORED).

## 5. Closures

### CLOS-MOVE-CAPTURE
**Rule.** A closure that escapes the function (returned, stored, passed to a thread) captures by move every variable
it names. A let-bound closure that never leaves the function borrows its captures (no `move`).
**Example.** `fn f(v: Vec<int>) -> int { let g = |i: int| v[i]; g(0) }` emits `let g = |i: i64| v[i as usize]`.
**Status.** Implementation-defined (no ADR); the escape test is `codegen/rust/closure_escape.rs`. Still over-owns the
captured parameter (D-CLOS-OVEROWN): `v` above is inferred `Vec<i64>` although the non-escaping closure only reads it.
Captures come from the liveness layer (`ir::liveness::captures`); a closure whose captured non-Copy binding is read
after the closure's statement (`ir::liveness::read_after`) borrows instead of moving, because `move` would be E0382
(`self.order.retain(|x| *x != k); self.order.push(k)`).

### CLOS-MUT-CAPTURE
**Rule.** A closure that mutates a captured variable needs the variable and the closure binding to be mutable.
**Status.** Diverges (D-CLOS-MUT): the generated Rust has neither `mut v` nor `let mut f` (rustc E0596).

## 5b. Structural typing (provenance)

These rules are not inference decisions; they name the structural constraints the solver receives so that every
constraint carries a rule (`wj explain-ownership` never prints `UNTAGGED`).

### TY-UNIFY
Operands of a comparison or arithmetic operator, a binding and its value, an assignment target and its value, a
returned value and the return slot, the elements of an array or the alternatives of an or-pattern have one type.

### TY-BOOL
Conditions (`if`, `while`, match guards), comparison results and logical operands are `bool`.

### TY-CALL-RESULT
A call's value has the callee's declared return type (`.to_string()`-style conversions produce `string`).

### TY-ANALYZER-SEED
Implementation seed: a parameter type the analyzer inferred (`inferred_param_types`). Not a language rule.

### VAL-OWNED
Constructed values (struct, map literals) and function results are owned values.

### EFF-CALL
Calls to effectful runtime functions contribute their effects to the caller (effect solver, not ownership).

## 6. Struct fields

### FIELD-OWNED
**Rule.** Struct, enum and tuple fields own their values. Constructing one from a variable moves the variable (or
clones it if it is used again, CLONE-USE-AFTER-MOVE). There are no reference fields.
**Status.** Conforms.

## 7. Numeric widths and text

Widths are decided by the numeric solver in principle (WP 8). Today four components decide them
(D-NUM-OWNERS). The rules below are ADR-0015's.

### NUM-DECLARED
**Rule.** A declared type (`x: u8`, `-> i32`, a struct field type) is exact. It is never widened or narrowed.
**Status.** Conforms.

### NUM-LITERAL-SUFFIX
**Rule.** A literal suffix (`5u8`, `1.5f32`) is exact.
**Status.** Conforms.

### NUM-PROPAGATE
**Rule.** An unsuffixed literal, and every variable bound to it, takes the width its uses require: passing it to a
`u8` formal makes it `u8`; adding it to an `i32` makes it `i32`. All values connected by assignment, comparison or
arithmetic form one class with one width (`+=`, loop accumulators included).
**Example.** `let a = 5; takes_u8(a)` emits `let a = 5_u8`.
**Status.** Diverges in one shape (D-NUM-FOR-ACC): `let mut u = 0; for x in vec![1,2] { u = u + x }` emits `u: i64`
and `x: i32` (rustc E0277).

### NUM-DEFAULT-INT
**Rule (ADR-0015 section 1).** An integer class with no constraint is `int` (i64).
**Why.** One default removes width churn; i64 does not overflow on common literals.
**Status.** Diverges (D-NUM-DEFAULT): the unified solver hard-codes `i32` for every unsuffixed literal and the legacy
integer solver defaults to `i32`. `let big = 3000000000` emits `3000000000_i32` (a rustc deny-by-default error).

### NUM-DEFAULT-FLOAT
**Rule (ADR-0015 section 1).** A float class with no constraint is `float` (f64).
**Status.** Diverges (D-NUM-DEFAULT): the unified solver says `f64`, the legacy float solver emits `f32`
(`let a = 2.5` emits `2.5_f32`), so the two disagree with each other as well as with the ADR.

### NUM-INDEX
**Rule (ADR-0015 section 1).** Indexing accepts `int` and `len()` returns `int`, converting through a checked
conversion. Explicit `usize` is only for FFI and layout.
**Status.** Diverges (D-NUM-OWNERS): the compiler currently emits `as usize` casts and `usize` locals, and a closure
body does not convert `len()` at all (D-NUM-CLOSURE-LEN).

### NUM-MIXED
**Rule (ADR-0015 section 2).** Mixed-width arithmetic requires an explicit conversion (`int(x)`, `float(x)`). The compiler
reports an error rather than inserting a cast.
**Status.** Not implemented: the compiler inserts implicit casts (`type_inference/int_implicit_casts.rs`). Accepted ADR,
unimplemented; sequenced with the numeric solver.

### NUM-CAST
**Rule.** `as T` fixes the width of its result to `T`; it does not constrain its operand.
**Status.** Conforms. (ADR-0015 section 2 replaces `as` with checked conversions; not implemented.)

### NUM-LITERAL
Non-numeric literal kinds (`bool`, `char`, `string`). Listed for provenance output only.

### NUM-CLASS
A value whose width is known (declared, suffixed, cast) is in the integer or float class of that width. Provenance
only: the class follows from the width.

### STR-OWNED
**Rule.** `string` is an owned, growable, UTF-8 string (`String`). It is never `&str` in a signature unless the
parameter is read-only and borrowed (OWN-STR-REF / STR-READONLY-REF).

### STR-LITERAL
**Rule.** A string literal is a `string` value. `let s = "a"` is a `String`; reusing `s` after a move follows
CLONE-USE-AFTER-MOVE. **Status.** Diverges (D-CLONE-STRLIT).

### STR-READONLY-REF
**Rule.** A read-only `string` parameter is borrowed as `&str`; callers may pass any string without cloning.
**Status.** Conforms (subject to D-OWN-PUB).

## 8. Local mutability and pins

### MUT-LOCAL-EXPLICIT
**Rule (ADR-0002, accepted).** A local binding is mutable only if declared `let mut`. Assigning to an immutable local
is a compile error with a `wj fix` suggestion that adds `mut`. Parameter and `self` *modes* stay inferred.
**Status.** Diverges (D-MUT-LOCAL): `let x = 1; x = 2` compiles and the compiler silently emits `let mut x`; so do
`mut self`, `mut v` for parameters, and `let mut` for a local mutated through a `&mut` call.

### Pins (OWN-PIN-REF, OWN-PIN-MUTREF)
There is exactly one pin mechanism today: `&T` / `&mut T` on a parameter and `&self` / `&mut self` on a receiver.
`owned` is not a keyword (ADR-0003 is Proposed). Clone and width have pins already (literal suffixes and declared
types; there is no way to forbid a clone). ADR-0016 proposes the extension.

## 9. Call sites

### CALL-BORROW
**Rule.** An argument for a `&T` formal is passed as `&x`; for a `&mut T` formal as `&mut x` (CALL-BORROW-MUT). The
caller never writes the borrow; the callee's formal decides.
**Status.** Conforms for same-file callees. Cross-file: the registry the call site reads can be one pass stale
(ratchet residue 4).

### CALL-BORROW-MUT
See CALL-BORROW.

### CALL-MOVE
**Rule.** An argument for an owned formal at the last use of the variable is passed bare.
**Status.** See MOVE-LAST-USE.

### CALL-CLONE
**Rule.** An argument for an owned formal when the variable is used afterwards (or in a loop) is `x.clone()`.
**Status.** See CLONE-USE-AFTER-MOVE and MOVE-LAST-USE.

## 10. What decides each rule today

| Decision | Intended owner (ADR-0001/WP) | Actual deciders |
|---|---|---|
| parameter mode | `ir::solver` | `analyzer::parameter_analysis` (+ `self_*`, `passthrough_inference`), seeded into the solver; seven IR post-passes; `codegen::function_generation_prepare::promote_*`; `generator::param_ownership_for_formal_demotion` merges analyzer and IR |
| clone | `ir::solver` (`NeedsClone`) | `auto_clone.rs`; ~2000 `.clone()` emission sites; the solver's result has no reader |
| integer width | `ir::numeric_solver` (WP 8) | unified solver default; `type_inference::int_inference`; `codegen::let_statement_generation`; `type_balancing` |
| float width | same | unified solver; `type_inference::float_inference` |
| local `mut` | the user (ADR-0002) | `codegen::let_statement_generation` (`auto_needs_mut`) |

Use `wj explain-ownership <file>` to see, per parameter, the analyzer, solver, IR and emitted modes side by side;
any row with differing columns is a place where the owners disagreed.
