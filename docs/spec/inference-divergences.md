# Inference divergences

Where the implementation disagrees with ADR-0001/0002/0015, with the spec (`inference.md`), or with itself.
Observed 2026-10-11 on the tree at commit `04c23fa8` (+ provenance tooling). Every repro was compiled with
`wj build --no-cargo`, and the generated Rust was fed to `rustc --edition 2021`; "rustc rejects" means that run failed.
Repros that rustc rejects are executable in `tests/inference_corpus/*.wj` (run by
`tests/inference_differential_test.rs`, listed in `.config/known-test-failures.txt`).

Severity: **S1** generated Rust does not compile; **S2** valid Rust, but behavior differs from the rule
(spurious clone, wrong width, over-ownership); **S3** structural (several owners, no user-visible symptom yet).

## D-OWNERS-OWN Parameter ownership has more than one owner (S3, ADR-0001)
ADR-0001 says only `OwnershipSolver::solve` writes ownership. In fact, for one parameter, in order:
1. `analyzer::parameter_analysis::infer_parameter_ownership` (a ~40-step precedence chain, plus `self_*` and
   `passthrough_inference`) fills `AnalyzedFunction::inferred_ownership`.
2. `ir::constraint_gen` feeds that result to the solver as `OwnershipIs` constraints (`OWN-ANALYZER-SEED`,
   `constraint_gen.rs` "Analyzer ownership as solver constraints") and again from `mutated_parameters`.
3. `IrFunction::from_analyzed` seeds the IR from the analyzer before solving, and `lower_to_ir` re-applies analyzer
   data after solving through `apply_analyzer_readonly_string_params_to_ir`, `clear_str_ref_for_owned_or_stored_params`,
   `restore_owned_string_params_after_str_ref`, `converge_impl_param_ownership`,
   `finalize_ir_param_ownership_from_analyzer`, `propagate_delegation_ownership`, `restore_mutated_param_mutrefs`.
4. Codegen merges again: `generator::param_ownership_for_formal_demotion` (analyzer vs IR, with explicit tie-breaks)
   and `function_generation_prepare::promote_*` (`promote_callee_forwarded_borrows`,
   `promote_unused_readonly_vec_params_to_borrowed`, ...), plus `compiler::multipass_bare_pass_demotion`.
Repro: `wj explain-ownership` on any file prints `stages: analyzer=.. solver=.. ir=.. emitted=..` per parameter; a
parameter whose stages differ (finding `D-OWN-STAGES`) was changed after the solver spoke.
Evidence the IR is not authoritative: `AnalyzedFunction::inferred_ownership` is documented "Legacy ... Superseded by IR
solver" yet is the seed and the override.

## D-SOLVER-UNION The solver drops ownership across type unions (S3)
`Solver::process_constraint(TypeEquals)` merges the *types* of two classes but not their ownership, and
`lower_to_ir` reads the result at `var.0` rather than at the class root. Consequence: `OwnershipIs` constraints that
reach a parameter through a union are invisible. This includes every call-site formal (`OWN-CALL-FORMAL`, created on a
fresh `expected_var` and joined to the argument by `TypeEquals`). `explain` marks such constraints `!` (dropped).
Repro (unit test, `#[ignore]`d until fixed): `src/ir/solver.rs`, `ownership_propagates_across_type_equals`.
Why not fixed here: the pipeline's later passes were written against the lossy result; fixing it changes the
solver's output for most functions, which needs the golden corpus (WP 0) as a gate. See convergence-plan step C2.
Related: `SolverResult::clones` is indexed by root as well, and nothing reads it (D-CLONE-OWNERS).

## D-CLONE-OWNERS Clones are not decided by the solver (S3, ADR-0001)
The solver's `NeedsClone` result (`CloneRequirements`) is computed and never read. `IrCutoverConfig.clones` switches
`auto_clone_analysis` between `analyzed.auto_clone_analysis` and `ir_fn.optimizations.auto_clone`, which is a copy of the
same analyzer data (`OptimizationHints::from_analyzed`). Clones are emitted by `auto_clone.rs` plus ~2000 `.clone()`
sites in `src/codegen` (`grep -rn "clone()" src/codegen | wc -l`). `constraint_gen::track_owned_use` (the solver's own
use counting) is never called, so `CLONE-MULTI-USE` is dead; `CLONE-ANALYZER` is the only live source.
Repro: `wj explain-ownership` prints `D-CLONE-OUTSIDE-SOLVER` for any variable cloned in the output without a solver
`NeedsClone`.

## D-CLONE-LASTUSE Clone at the last use (S2)
```windjammer
fn keep(v: Vec<int>) -> Vec<int> { v }
fn main() {
    let v = vec![1, 2, 3]
    println("${v.len()}")
    let w = keep(v)          // v never used again
    println("${w.len()}")
}
```
Emits `keep(v.clone())`. The smallest repro is `take(it); take(it)` (non-Copy `it`), which emits `take(it.clone()); take(it.clone())` on the current tree. Likewise `push_bag(b, "q"); let h = wrap(b, "t")` emits `wrap(b.clone(), ..)` although `b`
is dead, and arguments inside `println` interpolation are cloned (`field_str(cfg.clone())`). `explain` reports
`D-CLONE-LASTUSE` per call argument (rule `CALL-CLONE`, `used_later=false`). Not S1; costs allocations and hides
that ownership flowed. Cause: `auto_clone.rs` decides per statement index and treats a use after a `&mut` call or inside
a macro conservatively.

## D-CLONE-STRLIT A literal-bound string variable is assumed Copy (S1)
`tests/inference_corpus/string_literal_move_then_use.wj`:
```windjammer
let s = "str"
let t = s
println("${s} ${t}")      // rustc E0382: borrow of moved value `s`
```
`auto_clone` records `string_literal_vars` as "references, `.clone()` is a no-op", but codegen emits
`let s: String = String::from("str")`. Fix direction: STR-LITERAL says a literal is a `string` value, so the
exemption must go (or the binding must be emitted `&'static str`).

## D-CLONE-REDUNDANT Clone then borrow (S2)
`fn label(self, prefix: string) -> string { prefix + self.name }` emits `prefix + &self.name.clone()`: the clone is
allocated and immediately borrowed.

## D-SELF-VS-PARAM Same body, different mode for `self` and a parameter (S2)
`fn name_of(self) -> string { self.name }` emits `&self` plus `self.name.clone()` (SELF-GETTER) while
`fn name_of(b: Bag) -> string { b.name }` emits `b: Bag` and moves the field. Decided in two analyzers
(`self_return_and_consumption.rs`, `parameter_analysis.rs`).

## D-OWN-FIELD-RETURN-COPY A returned Copy field owns the whole parameter (S2)
`fn a(i: Item) -> int { i.qty }` emits `a(i: Item)`; `fn c(i: Item) -> int { i.qty + 1 }` and
`let q = i.qty; q` emit `&Item`. Callers of `a` clone `Item`. (`struct Item { name: string, qty: int }`)

## D-OWN-SHORT-TYPE-NAME A one-letter struct name is treated as a generic parameter (S2)
`struct B { items: Vec<string> }  fn n(b: B) -> int { b.items.len() }`: `analyzer::generic_analysis::is_generic_type_param`
treats any single capital letter (or letter+digit) as a type parameter and pins the parameter `Owned` (step 0a);
codegen later demotes it to `&B`, so the analyzer and the emitted signature disagree (`D-OWN-STAGES`). Type names
should resolve to definitions (WP 6/7), not be guessed by spelling.

## D-NUM-INTERP-CALL Calls inside `${...}` did not constrain literals (S1, fixed here)
Expressions inside interpolation were lexed as a fragment, so their positions were relative to the fragment (1:1,
1:5, ...). Numeric inference keys expressions by position, so every interpolation collided with every other and with
real expressions on line 1; `println("${read_vec(v0)}")` did not make `vec![1, 2, 3]` an `i64` vector
(`tests/inference_corpus/interp_call_width.wj`, rustc E0308). Fix: `parser/interpolated_string_expression_parser.rs`
re-bases sub-expression positions onto the enclosing string (`tests/interpolation_expr_location_test.rs`). The same
collision fed `auto_clone` and diagnostics (D-CLONE-LASTUSE for arguments inside interpolation is a probable
consequence; re-measure after this change).

## D-OWN-OPTION-MATCH Matching an `Option` by value owns it (S2)
`fn f(o: Option<string>) -> int { match o { Some(s) => s.len(), None => 0 } }` emits `o: Option<String>`; the
read-only `o.is_some()` emits `&Option<String>`.

## D-OWN-STORE-ASSIGN Stored by assignment is borrowed and copied; stored by push is owned (S2)
`fn store_name(h: Holder, n: string) { h.tag = n }` emits `n: &str` and `h.tag = n.to_string()`, while
`fn push_bag(b: Bag, s: string) { b.items.push(s) }` emits `s: String`. Two models for "parameter stored in a
field". This is the question behind `docs/known-issues/ownership-and-width-ratchet-residue.md` section 2 (the
`path_and_bytes_borrow_tests` entries); an ADR must pick one before either side is changed.

## D-OWN-PUB Visibility changes parameter mode (S2)
`fn unused(v: Vec<int>, s: string) {}` emits `_v: Vec<i64>, s: &str`; the `pub` version emits `s: String`
(`parameter_analysis.rs`, "Unused **pub** `string` formals keep Owned"). A signature's mode depends on a keyword that
has no ownership meaning; the stated reason (API stability) is the use case for a pin (ADR-0016).

## D-CLOS-OVEROWN Read-only closure capture owns the parameter (S2)
`fn use_closure(v: Vec<int>) -> int { let f = |i: int| v[i]; f(0) + f(1) }` emits `v: Vec<i64>` and `move |i| v[i]`.
The closure only reads `v`.

## D-CLOS-MUT Closure that mutates a capture (S1)
`tests/inference_corpus/closure_mut_capture.wj`: `let mut f = |i: int| v.push(i)` inside `fn fill(v: Vec<int>)`
emits `v: Vec<i64>` and `let f = move |i| v.push(i)`; rustc E0596 twice (`v` and `f` need `mut`).

## D-PIN-DOUBLE-REF Explicit `&Vec<T>` is emitted as `&&[T]` (S1)
`tests/inference_corpus/pin_ref_vec.wj`: `fn count(v: &Vec<int>)` emits `v: &&[i64]` and the call site passes `&v`
(rustc E0308). The user's `&` and the compiler's own read-only demotion both apply.

## D-PIN-NOT-HONORED A pin that contradicts the body is not diagnosed (S1)
`fn f(v: &Vec<int>) -> Vec<int> { v }` emits `v: &[i64]` returning `Vec<i64>` (rustc E0308) instead of a
Windjammer error at the annotation. OWN-PIN-* requires an error.

## D-MUT-LOCAL Local mutability is inferred, not explicit (S2, ADR-0002)
ADR-0002 (accepted, option B): `let x = 1; x = 2` is an error with a `wj fix`. Today it compiles and emits
`let mut x`. Same for `mut self`, `mut v` parameters, and `let mut` on a local whose only mutation is a `&mut`
call. `let_statement_generation.rs` (`auto_needs_mut`), `function_formal_parameter_generation.rs`,
`variable_analysis/mut_inference.rs` implement the opposite of the ADR.

## D-NUM-DEFAULT Unconstrained numerics are i32 / f32 (S1 for big literals, S2 otherwise, ADR-0015)
`let big = 3000000000` emits `3000000000_i32` (`tests/inference_corpus/default_int_literal.wj`, rustc: literal out of
range). `let a = 2.5` emits `2.5_f32`. The unified solver hard-codes `Literal::Int -> I32`
(`constraint_gen::literal_to_base_type`, `solver::resolve_numeric_classes`), and the legacy float solver defaults to `f32`
while the unified solver says `f64`; ADR-0015 says `int`=i64, `float`=f64.

## D-NUM-OWNERS Width has four deciders (S3, WP 8)
Unified solver (`ir/solver.rs`), `ir/numeric_solver.rs` (not authoritative), legacy `type_inference/{int,float}_inference`,
and codegen heuristics (`let_statement_generation.rs` `prefer_usize_over_i32`, `while_i32_peer`, `type_balancing.rs`,
`int_promotion_type_inference.rs`). `explain` prints `D-WIDTH-OWNERS` when the solver width of a local differs from the
emitted one. Repro of the ratchet entry: `let mut i = 0; while i < 6 { best = offsets[i]; i = i + 1 }` emits
`i: i32` and `offsets[(i as usize)]`.

## D-NUM-FOR-ACC Accumulator and element widths differ (S1)
`tests/inference_corpus/for_accumulator_width.wj`:
```windjammer
let mut u = 0
let w = vec![1, 2]
for x in w { u = u + x }
```
emits `u: i64`, `w: Vec<i32>` (rustc E0277: `i64 += i32`). NUM-PROPAGATE requires one class.

## D-NUM-CLOSURE-LEN `len()` in a closure body is not converted (S1)
`tests/inference_corpus/closure_len_width.wj`: `|x: int| x + v.len()` emits `x + v.len()` (`i64 + usize`); the same
expression outside a closure gets `as i64`. The conversion is applied in expression generation, which does not reach
closure bodies.

## D-INTERP-CLOSURE-NONE Interpreter: closure calls evaluate to None (S2, not inference)
`let f = |x: int| x + 1; println("${f(1)}")` prints `None` under `wj run --interpret` on the reference build. Found by the
differential harness; belongs to the interpreter stream. The two closure corpus programs are therefore judged by rustc only.

## Findings emitted by `wj explain-ownership` (generic)
### D-OWN-STAGES Parameter mode changed between pipeline stages
The analyzer, solver, IR and emitted modes are not all equal (see D-OWNERS-OWN).
### D-CLONE-OUTSIDE-SOLVER Emitted clone with no solver clone constraint
See D-CLONE-OWNERS.
### D-WIDTH-OWNERS Solver width differs from emitted width
See D-NUM-OWNERS.
