---
status: current
last_verified: e0ec926dc
---
# Numeric widths: removing the legacy deciders instead of modelling their knowledge

Design review for the owner's question: the C4 width convergence keeps needing the legacy codegen deciders (W3-W6,
`docs/spec/convergence-burndown.md`) next to the IR solver, because the solver is a numeric side-system without type
facts. Is there an approach that deletes the deciders instead of re-modelling their knowledge one fact class at a time?

Short answer: yes. Two of the three hard problems the solver is trying to solve are not inference problems at all.

1. **`usize` is a Rust representation, not a Windjammer type.** ADR-0015 section 1 (accepted) already says indexing
   and `len()` accept and return `int` through a checked conversion; `docs/spec/inference.md` NUM-INDEX lists the
   compiler as *diverging* from that rule. ADR-0028 section 4 kept `usize` as an inferred class "until it meets an
   `int`", which is what made the solver need receiver types, element types and std argument types. A local boundary
   rule (`int` everywhere in the language, `usize` only at the Rust sinks and sources) needs none of them.
2. **Unsuffixed literals are already polymorphic in Rust.** rustc is a complete type checker with integer and float
   literal variables. The compiler only has to suffix a literal when rustc's fallback (`i32`, `f64`) would differ from
   Windjammer's default (`int` = i64, `float` = f64). For floats it never differs. For integers it differs only in a
   class with no constraint at all, and that is the one case the solver can decide alone (a *closed* class).

The third problem (receiver and element types for ownership, method dispatch and diagnostics) is real type
inference. It is worth doing (option C), but widths should not wait for it.

Recommendation: **B + D2** ("int at the surface, rustc types the literals"), staged so each step deletes a legacy
decider, followed by **C** as the long-term owner of every other type fact. Option A is stopped.

## 1. Evidence: what the current path costs

All numbers measured at `e0ec926dc` (`next/v0.50.0`) in this worktree unless noted.

### 1.1 The deciders and their size

| decider | file(s) | lines | `usize` / `i32` lines |
|---|---|---|---|
| W3 peer/promotion thicket | `codegen/rust/binary_expression_generation.rs` | 1573 | 122 / 77 |
| W4 compare promotion | `codegen/rust/int_promotion_type_inference.rs` | 2183 | 73 / 151 |
| W5 usize inference and casts | `codegen/rust/usize_expression_type_inference.rs` | 1163 | 169 / 66 |
| W6 untyped-let paints | `codegen/rust/let_statement_generation.rs` (~600 of) | 1621 | 51 / 38 |
| W7 call-site numeric coercion | `codegen/rust/type_casting.rs` | 730 | 99 / 39 |
| W8 legacy collectors | `type_inference/int_inference/` + `float_inference/` | 2712 + 3301 | |
| W9 class defaults | `ir/numeric_defaults.rs` | 561 | |
| W10 balancing and implicit casts | `codegen/rust/type_balancing.rs`, `type_inference/int_implicit_casts.rs`, `int_formal_scan_extra.rs` | 1180 + 191 + 38 | |
| W12 call-site arg coercion | `codegen/rust/ir_call_site.rs` (part of 10444) | ~1500 | 117 / 14 |

Behind all of them sits a de-facto type checker in codegen: `ExpressionTypeInference::infer_expression_type`
(`codegen/rust/expression_type_inference.rs`, 1094 lines) is called from **307 sites in 45 files**. The deciders "know
things the solver does not" because they consult this second, ad-hoc typer. The solver side (`ir/solver.rs` 1044,
`ir/constraint_gen.rs` 1971, `ir/constraint_gen/numeric.rs` 448, `ir/numeric_facts.rs` 215,
`codegen/rust/solver_widths.rs` 124) re-derives a subset of the same knowledge a second time.

### 1.2 Churn and regressions caused by piecemeal modelling

- 62 non-merge commits since 2026-10-01 touched the four W3-W6 files; 202 commits in the history mention `usize`,
  `i32` or width.
- Burn-down pace log: across nine C4 steps, 271 lines deleted against ~1200 added; `D-WIDTH-OWNERS` moved only through
  the usize class (81 -> 58) and then stalled at 58; the kill criterion fired on 2026-11-02.
- Regression 1 (`docs/drift/findings.md` section C): publishing usize widths broke nine ecosystem targets
  (263 test cases) because the corpus lacked `len() - 1` returned as `int`, an `int` compared with `len()`, and a
  `-1` tail. The fix (`7c5067ace`) added five more "held class" reasons, each a fact class the solver cannot see.
- Regression 2 (the "publish defaulted classes" experiment, `WJ_EXP_PUBLISH_DEFAULTED` in the paused WIP commit
  `c32bda5c4`): 92 goldens changed, 31 new rustc rejects. That WIP commit already adds 582 lines
  (`constraint_gen/collections.rs` 337) for *one* fact class (collection elements and closures), unverified.
- Drift findings section A: nine interpreter-green / compiled-red programs, all of the shape "the backend picked `i32`
  for an unsuffixed literal whose context is `int`" (`Some(b + 1)` with `b: int`, a literal captured by a closure,
  `Some(7)` flowing into an `Option<int>` parameter).

### 1.3 What the goldens say (independent oracle: rustc)

`rustc --emit=metadata` over the 324 golden Rust files (`tests/golden/rust`, the same check as gate 2), collecting
*all* errors rather than the first: 86 rejected plus 5 whose build failed (= the 91 entries of
`tests/golden/rustc_rejects.txt`; rustc accepts the other 233 golden files). **18 rejected goldens contain a numeric-width error**:

| error | count |
|---|---|
| expected `i64`, found `i32` | 12 |
| expected `f32`, found `f64` | 6 |
| expected `usize`, found `i64` / `i32` | 7 |
| expected `i64`, found `usize`; `cannot add usize to i64`; iterator over `(String, usize)` | 4 |
| expected `&i64`, found `i64` (ownership, not width) | 4 |
| ambiguous numeric type `{integer}` (E0689) | 2 |
| `Extend<i32>` / iterator over `i32` | 3 |

Examples: `1_i64 + height(*l).max((height(*r) as i32))` (a cast the legacy path invented), `vec![vec![1_i32, 2_i32]]`
passed to a `Vec<Vec<i64>>` formal, `v.iter().take(3_i64)`, `vec![false; 5_i32]`.

The emitted Rust carries 1084 `_i64`, **359 `_i32`**, 167 `_f64`, **133 `_f32`** and 102 `_usize` literal suffixes.
Of the 66 goldens that emit `i32` anywhere, **29 never mention `i32` in their source**: the paint invents a width the
language does not have.

### 1.4 Pre-experiment without building the compiler (rustc as the literal typer)

Throwaway probe (scripts in the session scratchpad, not committed): strip every literal suffix from the 324 goldens
with one regex, leave every cast and annotation, re-run rustc.

- **7 goldens become accepted** (`algo_matrix_multiply`, `iter_take_skip_rev`, `prog_game_of_life`,
  `loop_index_type_inference`, `tuple_enum_variants`, `conformance/error_handling_result_and_option`,
  `conformance/stdlib_closures_and_iteration`).
- 6 goldens break, in exactly two shapes the solver can see without type facts:
  a literal (or a local bound only to literals) used as a **method receiver** (`(25.0).sqrt()`, `nan.is_nan()`,
  `x.floor()`, `x.powi(..)`, `a.partial_cmp(&b)` over a vector of literals: E0689/E0277, 5 goldens), and a
  **closed class** whose value does not fit rustc's `i32` fallback (`literal out of range for i32`, 1 golden). Keeping
  the suffix on a `let` bound to a lone literal already fixes the second (second variant of the probe).

A blunt regex with zero type knowledge fixes more goldens than the last four C4 steps together, and every break it
causes is syntactic. That is the core evidence for D2.

### 1.5 Performance of `int` indices (LLVM)

Probe (`rustc -C opt-level=3`, aarch64, scratchpad): summing a `Vec<i64>` with a `usize` counter, with an `i64`
counter and `v[i as usize]`, and with an `i64` counter through a checked `usize::try_from` helper. LLVM's identical-code
folding emits **one function for all three** (`_sum_usize = _sum_int_checked`, `_sum_int_for = _sum_int_checked`);
the `while` forms fold the same way (`_sum_usize_while = _sum_int_while`). A 200x200 triple-loop matrix product has the
same four bounds checks in both versions and differs only in one loop-entry comparison (`n >= 1` vs `n != 0`);
timings are within noise (load average 11.6 during the run). The checked conversion at an index is free in the loops
that matter; it is a branch where the value enters the index.

### 1.6 The interpreter already wants B

- `interpreter/int.rs`: `IntTy::unify` lets the left operand's width win, so `i64 + usize` already runs; the only
  observable difference is the overflow range (ADR-0028 section 5: `v.len() - 1` on an empty vector is an error
  interpreted and wraps compiled).
- `interpreter/resolved/typed.rs`: the statically typed fast path (`ICode` yielding raw `i64`) **excludes `usize`**
  (`!matches!(t, IntTy::U64 | IntTy::Usize | ...)`), so every length and index computation today stays on the slow
  `Value` path. Under B they are `int` and qualify.
- `std/` disagrees with itself: `std/strings.wj` and `std/json.wj` declare `len -> usize`, while `collections.wj`,
  `map.wj`, `regex.wj` and `fs.wj` declare `len`/`size -> int`.
- The Go backend's `len()` is `int` (64-bit) and JS has numbers: only the Rust backend has a `usize`.

## 2. Options

### A. Keep modelling facts in the numeric solver (current path)

**Missing fact classes.** The held-class reasons in `ir/numeric_facts.rs::held_classes` and
`constraint_gen/numeric.rs::Unmodelled`, the burn-down and the drift ledger name these, each one a slice of a type
checker: (1) closure parameters and returns, (2) `Vec<T>` element flow (push/insert/index/iter/extend/collect),
(3) `HashMap`/`HashSet`/`BTreeMap` key and value flow (get/insert/entry/values/keys), (4) std method argument types,
(5) std method result types (beyond `len`/`count`), (6) iterator adaptor chains (map/filter/fold/zip/enumerate, element
types through closures), (7) `Option`/`Result` payloads through match/if-let/`?`/`??`, (8) tuple elements and
destructuring, (9) cross-module and generic struct fields (P0 done for direct fields only), (10) receiver types of
method calls (homonym `len`), (11) generic function instantiation, (12) trait method signatures, (13) module
constants and statics, (14) Rust crate surfaces (`.wji`). Shadowing and `as` casts are done.

**Remaining work.** The one class attempted (2 + part of 1, WIP `c32bda5c4`) is 337-582 lines and unverified. At that
rate: 12 classes x ~400 lines = **~5k lines of solver code**, i.e. a second type checker inside the numeric solver,
before W3/W4/W6 can be deleted (each decider consults several classes, so nothing is deletable until nearly all are
done). Each class has needed one or more regression rounds so far (section 1.2). The result duplicates
`infer_expression_type` rather than replacing it.

- Deletes: eventually W2-W12 (~15k), but **0 lines until the last class lands**.
- Adds: ~5k (solver) on top of the 4.7k solver/constraint code already there.
- Risk: high and recurring: every unmodelled shape is a silent wrong width that rustc rejects or, worse, accepts with a
  different overflow range. The corpus is the only defence, and section 1.2 shows it misses real shapes.
- Kill criterion (already met): two steps without lowering `D-WIDTH-OWNERS`.

### B. `usize` is invisible: `int` at the surface, a boundary rule at the Rust sinks

**Language rule** (ADR-0015 section 1, made literal): `len()`, `count()`, `capacity()`, `position()`, `enumerate()`
indices, `find()` byte offsets and every other std or crate result that Rust spells `usize` are `int` in Windjammer.
Indexing, slicing and every std or crate formal that Rust spells `usize` accept `int`. A declared `usize` in user code
remains an exact type (FFI, layout) and follows ADR-0015's no-implicit-conversion rule.

**Backend rule** (local, deterministic, no inference):

| site | emitted Rust |
|---|---|
| source: `x.len()` (and the source table) when no user signature for `len` resolves | `(x.len() as i64)` |
| sink: `v[e]`, `&s[a..b]`, std/crate `usize` formal, `vec![x; n]`, `Vec::with_capacity(n)` | `v[wj_ix(e)]`, `take(wj_ix(n))` |
| `for (i, x) in v.iter().enumerate()` | the index binding is converted at the pattern (`i as i64`) |
| literal at a sink | unsuffixed (`take(3)`): rustc infers `usize` |

`wj_ix` is a three-line generic helper in the generated prelude:
`#[inline(always)] #[track_caller] fn wj_ix<T: TryInto<usize> + Copy + Display>(t: T) -> usize`, panicking with
`index -1 is negative`. Because it is generic over the integer type, **the sink needs no width knowledge at all**: it
accepts `i64`, `i32`, `u32`, `usize` or an unsuffixed literal (verified with rustc in the probe: `take(wj_ix(n))` with
`let n = 2`, `v[wj_ix(k)]` with `let k = 0`, `vec![vec![false; wj_ix(5)]; wj_ix(3)]`, a negative `i64` panics). The
only receiver fact still needed is "is this a sequence or a map index", which the backend already decides today to
emit `&k` for maps.

**Semantics.** Negative index: one defined panic in both engines (`index -1 is negative`), instead of
"index out of bounds: the len is 3 but the index is 18446744073709551615". Underflow: `v.len() - 1` on an empty vector
is `-1`, ordinary signed arithmetic in both engines; the ADR-0028 section 5 interpreter/compiled divergence
disappears. Overflow: only one rule left (`int` overflow, ADR-0015), so enabling `overflow-checks` in the generated
release profile becomes a single, measurable decision.

**Performance.** Section 1.5: identical machine code for the index loops that dominate; one branch where a value
enters an index. Optional, later: a peephole that keeps `for i in 0..v.len()` native `usize` when `i` is used only as
an index (both widths hold every value in `0..len`, so it is an optimization, not a decision) - only if a benchmark
asks for it; the probe says it will not.

**Interop (ADR-0026).** `.wji` surfaces keep exact widths for `u8`/`u32`/`f32`/`i32`. `usize` in a surface means "index
slot": `int` on the Windjammer side, `wj_ix` at the formal, `as i64` at the result. `levenshtein(a, b) -> usize`
becomes an `int` to the caller. A crate that needs a true `usize` value (bit patterns, hashes) declares `u64`.

**Readability.** Fewer, uniform conversions: today the goldens contain 49 `as usize`, 52 `as i64`, 22 `as i32`, 102
`_usize` and 359 `_i32` suffixes placed by four deciders; under B every conversion is `wj_ix(..)` at a sink or
`as i64` at a source, and a reader can predict each one from the line alone.

- Deletes outright: **W5** (1163), the ADR-0028 machinery (`PrefersUsize`, `index_sources`, `index_uses`,
  `element_stores`, `usize_conversions`, the index and `unknown_lengths` hold reasons, `BaseType::Usize` as an
  inferred class: ~350 lines across `solver.rs`, `constraint_gen/numeric.rs`, `numeric_facts.rs`,
  `solver_widths.rs`), `int_implicit_casts.rs` (191), the usize branches of W3 (~300), W7 (~150) and W12 (~400).
  **~2.5k lines.** It does not delete the `i32` paints; D2 does.
- Adds: source/sink table (one data table, derived from `stdlib_surface::known_methods` plus Rust's `usize` positions,
  ~60 entries), `wj_ix` (3 lines), interpreter change (`Value::usize` callers return `int`, 19 `IntTy::Usize` sites
  reviewed), spec/std edits (`strings.wj`, `json.wj` `len -> int`). ~300 lines.
- Risk: medium-low. Behaviour change only for programs that relied on `usize` wrap-around (none in the corpus; to be
  checked over the ecosystem). The goldens change widely (every `len()` site), which is a review cost, not a risk:
  gate 2 (rustc) and the drift differential corpus are the oracles.

### C. One type owner

**What exists.** There is no type checker to reuse. `check/semantic` is documented as "conservative AST-level
semantic checks that need no type checker" (`check/semantic/mod.rs`); its notion of type is a `Kind` for literal
mismatches and a struct index. `std_methods.rs` is a *name* table (`KNOWN_METHODS`, 104 entries, no types). What does
exist is scattered typing: `infer_expression_type` (307 call sites), `ir/expr_static_type.rs::declared_type_of`
(parameter/field/index chains), `ir/receiver_type.rs`, `type_inference/struct_field_registry.rs`, the signature
registry (`ir/signature_bridge.rs`, 1859), the analyzer's local types, and the interpreter's own static width typing
(`interpreter/resolved/typed.rs`). The std surface in `std/**/*.wj` is typed Windjammer (44 files); Rust std
(Vec/HashMap/iterators/String) has no typed surface.

**Design.** A bidirectional local type inference per function (declared signatures are mandatory, so it is
check-mode almost everywhere; synthesis only for `let` and closures), with integer/float literal variables defaulting
to `int`/`float`, generic instantiation for std containers, `Option`/`Result`, iterators and closures; the Rust std
surface as typed data (a new interface file for the Rust core library, in the same `.wji` format ADR-0026 uses for crates); results stored
on the IR keyed by node id (ADR-0017's semantic facts). Widths are then just types; the numeric solver keeps only
ownership; the interpreter's resolved engine reads the same types for its fast path; `wj check` reports type errors
in Windjammer terms before rustc.

- Deletes: `infer_expression_type` and its 307 call sites, `expr_static_type`, `receiver_type`,
  `match_binding_type_inference.rs` (433), `type_name_inference.rs` (339), W11/W12 formal scans (~2.3k), most of the
  numeric solver, and it unblocks O4/O6 (ownership needs receiver types too). With B+D2 done first: ~6-8k more.
- Adds: 8-15k lines (checker 4-6k, typed Rust-std surface 2-4k entries-as-data, diagnostics 1-2k, migration glue).
- Risk: high for the schedule, low for correctness once landed (soundness is checkable against rustc on every
  golden: the checker's type for every expression must match what rustc infers - an oracle that can be automated with
  `rustc -Zunpretty=thir-tree` or by emitting type ascriptions and requiring rustc to accept them).
- Staging: (C1) typed Rust-std surface as data, gated by a test that every entry resolves in rustc; (C2) the checker in
  report-only mode, ratchet "expressions without a type" over the corpus; (C3) codegen reads receiver types from it,
  delete `infer_expression_type` call sites file by file; (C4) ownership reads it (O4/O6). Kill criterion per stage:
  the "untyped expressions" ratchet must fall every step; any step where the checker disagrees with rustc on more than
  1% of corpus expressions stops for redesign.

### D. Other approaches

**D1. Everything `i64`, `.try_into().unwrap()` at sinks.** This is B's sink rule with a different spelling; the generic
helper `wj_ix` is the better form (track_caller location, Windjammer message, cold path). Folded into B.

**D2. rustc types the literals.** The backend stops choosing literal widths except in two syntactic cases:

| solver verdict for the literal's class | emitted |
|---|---|
| exact width W (declared, suffix, `as`, field, formal) | `5_W` or unsuffixed (both correct) |
| *closed*: every member's use is modelled and none has a width | `5_i64` / `2.0` (float fallback is already f64) |
| *open*: some use the solver cannot see | **unsuffixed**: rustc infers from the use |
| a literal-only class used as a **method receiver** (`(25.0).sqrt()`, `x.abs()`) | default suffix (`_f64`, `_i64`) unless an exact width is in the class |

The polarity flip is the point: today "held" means *the legacy deciders decide*; under D2 it means *rustc decides*,
and rustc has every fact the solver lacks (closure parameters, element types, std formals, receivers, crate
signatures). The publish-defaulted experiment failed because it pinned open classes to `i64`; D2 pins only closed
classes. Residual risk: an open class whose only unseen use is itself generic (a value passed through `println!` or a
generic function) gets rustc's `i32` fallback; that is a silent overflow-range divergence, detectable by the drift
differential corpus with values above 2^31 and by a solver count of "open classes with no exact member" (the
experiment measures it).

- Deletes: **W2** (literal collector fallback, ~120), **W3** peer thicket (~600 after B), **W4**
  `int_promotion_type_inference.rs` (2183), **W6** let paints (~600), **W8** int collector (2712) and the whole float
  collector (3301; float literals need no decision at all), **W9** folds into the closed-class test (561), W10
  balancing for literals (part of 1180). **~9-10k lines.**
- Adds: the three-way verdict in `numeric_facts.rs` and the receiver rule (~150 lines).
- Risk: medium. rustc's error for a genuinely ambiguous program is E0282/E0689 "type annotations needed", which maps to a
  Windjammer diagnostic with a precise fix (`annotate the binding: let x: float = 2.0`).

**D3. rustc in the loop (repair or read-back).** Compile, parse `--error-format=json` E0308 on integer types, insert
the conversion, recompile; or read inferred types back (`-Zunpretty=thir-tree`, nightly). Path: a build flag for width repair that converges in at most two rounds, behind a flag, measured on the ecosystem. Not first because it
doubles build time, needs nightly for read-back, and masks real user errors as auto-casts (ADR-0015 forbids implicit
narrowing). Useful later as the *oracle* for C (does the checker agree with rustc?), not as the mechanism.

**D4. A Windjammer prelude that lets rustc dispatch on types.** Traits such as `WjLen` (`fn wj_len(&self) -> i64` for
Vec/slice/str/String/HashMap/HashSet/VecDeque/BTreeMap) remove even the "no user `len`" receiver check from B, because
rustc resolves the receiver. Same idea could cover `enumerate` and `position`. Cheap and compatible with B; use it
where a receiver check would otherwise be needed. (A trait-based `Index` for maps and sequences runs into place
expressions and coherence; not proposed.)

## 3. Comparison

| | A: model facts in solver | B: usize boundary | D2: rustc types literals | B + D2 | C: one type owner |
|---|---|---|---|---|---|
| legacy deciders deleted | W2-W12 eventually, none until ~12 classes land | W5, ADR-0028 machinery, usize parts of W3/W7/W12, `int_implicit_casts` | W2, W3, W4, W6, W8 (int+float), W9 | W2-W10 and the usize half of W12 | `infer_expression_type` (307 sites), W11/W12, `expr_static_type`, `receiver_type`, unblocks O4/O6 |
| LOC deleted (est.) | ~15k, late | ~2.5k | ~9-10k | **~12k** | +6-8k after B+D2 |
| LOC added (est.) | ~5k | ~300 | ~150 | ~450 | 8-15k |
| risk | high, recurring (2 regressions so far) | medium-low; semantic change for usize wrap | medium; residual i32 fallback in open classes | medium | high schedule, low correctness |
| staging | per fact class, no deletion per step | 3 steps, each deletes | 2 steps, each deletes | 5 steps (section 4) | 4 steps (C1-C4) |
| width errors agents see | Rust widths they never wrote (`usize`, `i32`) | only widths they declared | rustc "annotation needed" mapped to a Windjammer fix | only declared widths, plus rare "annotate this binding" | Windjammer type errors before rustc |
| first-pass compile (golden files rustc accepts: 233/324 today) | +2 per C4 step so far (93 -> 91) | est. +6 (usize-shaped rejects) | +7 measured by the regex probe, 6 breaks all in the two D2 rules | est. +12-14, width rejects 18 -> <= 4 | all remaining type-shaped rejects |
| interpreter | unchanged | `len` returns `int`; usize leaves the slow path; underflow divergence gone | unchanged (`Lit` already adopts) | both | resolved engine can read static types |
| other backends | Go/JS re-derive widths | Go/JS already int | no effect | simpler portable lowering | one fact source (ADR-0017) |

## 4. Recommendation and staged migration (B + D2, then C)

Stop option A (its kill criterion has fired; the paused WIP `c32bda5c4` should not be resumed). Adopt B + D2 under a new
ADR that supersedes ADR-0028 (draft: `docs/adr/0029-int-at-the-surface.md`). Each step is one branch, deletes code,
and must keep gate 2 (rustc rejects may only fall), the narrowing-cast gate, `scripts/ecosystem_width_gate.wj` (9/9)
and the drift differential corpus green.

| step | change | deletes | kill criterion |
|---|---|---|---|
| S0 | experiment (section 5), env-gated, throwaway | nothing | see section 5 |
| S1 | floats: emit float literals unsuffixed except the method-receiver rule and exact widths | float collector (3301), float parts of W3/W6 | any new rustc reject other than E0689 on a receiver the rule missed; any drift divergence |
| S2 | integers: closed/open/exact verdict; open classes unsuffixed; receiver rule | W2, W6 paints, W4, W3 peer thicket, int collector (2712), W9 | rejects rise; "open classes with no exact member" > 5% of integer classes in the corpus; any differential-corpus overflow divergence |
| S3 | usize boundary: source table + `wj_ix` sinks, `len -> int` in std and interpreter, ADR-0028 superseded | W5, ADR-0028 solver machinery, `int_implicit_casts`, usize branches of W3/W7/W12 | ecosystem gate below 9/9 after table fixes; interpreter/compiled divergence on `len_index_arith.wj` or the drift corpus; > 2% runtime regression in the bench suite |
| S4 | `D-WIDTH-OWNERS` redefined (solver verdict vs emitted, open counts as agreement) and ratcheted to 0 | the meter's spelling special cases | meter cannot reach 0 without re-adding a decider |
| S5 | C1-C4 (one type owner) for receivers, ownership and diagnostics | `infer_expression_type` and the rest | per-stage ratchets in option C |

S1 first because it is the safest: rustc's float fallback *is* Windjammer's default, so float literals never need a
decision, and it deletes the largest single collector. S3 is the language-visible step and needs the owner's
acceptance of the ADR.

## 5. First experiment (under a day of build time)

Branch `wt/width-exp` from `next/v0.50.0`, one env toggle `WJ_EXP_INT_SURFACE=1`, one release build of `wj` through
the build slot, then the existing gates. No deletions; the toggle only changes what the backend prints.

1. `ir/numeric_facts.rs`: under the toggle, classify each literal's class as exact / closed / open (closed = defaulted
   and not held; open = held). Publish closed classes as `i64`, mark open literals in a new `unsuffixed_literals` set.
2. Rust backend (literal emission and the let path): an open literal prints with no suffix and the let gets no
   ascription; a literal-only class that is a method receiver keeps the default suffix. W2/W3/W4/W6 are skipped for
   those literals and locals.
3. Disable `PrefersUsize` (the index class) under the toggle; emit `(x.len() as i64)` for `len`/`count`/`capacity`
   with no user signature, `wj_ix(..)` at every sequence index, slice bound and the std `usize` formals table
   (start from `take skip nth step_by chunks windows split_at truncate insert remove swap swap_remove resize
   with_capacity reserve repeat rotate_left rotate_right split_off get get_mut`), and convert `enumerate` indices.
   Add `wj_ix` to the generated prelude.
4. Measure, toggle off vs on:
   - gate 2 over the 321 goldens: reject count (91), and the 18 numeric-width rejects of section 1.3;
   - `scripts/ecosystem_width_gate.wj` (9 targets) and the drift differential corpus, compiled;
   - the drift section A repros (nine literal findings) and `tests/interpreter_corpus/len_index_arith.wj`;
   - count of open integer classes with no exact member (the i32-fallback exposure);
   - `D-WIDTH-OWNERS` with open counted as agreement.

**Confirms the recommendation if:** rejects fall by at least 8 (91 -> <= 83), width-error rejects fall from 18 to
<= 6, no golden that rustc accepts today is rejected, ecosystem gate 9/9, at least 6 of the 8 open drift section A
repros pass compiled, and open classes with no exact member are <= 5% of integer classes.

**Kills it if:** more than 3 newly rejected goldens need a type fact (an E0282/E0283/E0689 the receiver rule cannot
cover, or a `usize` sink missing from a table that cannot be closed by adding entries), or the differential corpus shows
an overflow-range divergence caused by rustc's `i32` fallback that the closed-class rule cannot remove. In that case
D2 falls back to "open classes keep the legacy decision" and the review goes straight to C, with B kept (B does not
depend on D2).

## 6. Effect on agents

Agents write `let last = v.len() - 1`, compare an `int` with `len()`, pass a counter to `take(n)` and push an index into a
`Vec<int>`: today each is a potential rustc E0308 about `usize`, mapped to `WJ0003` with Rust types the agent never wrote,
and the fix the agent reaches for (`as usize`) is the Rust idiom ADR-0014 retires. Under B these programs are simply
correct in both engines. Under D2 the only new diagnostic is "annotate this binding" (`let x: float = 2.0`), with an
exact span and a machine-applicable fix. The width errors that remain are about widths the program declared
(`u32` field, `f32` vertex), which is the honest place for them.
