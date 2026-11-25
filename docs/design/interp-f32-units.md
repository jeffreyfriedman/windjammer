---
status: proposal
last_verified: a0f9b9daca72
---
# f32 arithmetic in machine-code units

> **Proposal: not implemented.** This is the f32 rung that comes before the inline-callee ladder
> ([interp-inline-callee.md](interp-inline-callee.md)) for game-shaped code. File and line references are to
> `wt-rc3-int` a0f9b9dac. Measured numbers are quoted from pc-results measurement files, which are named at each use:
> `findings/interp-e1-e6-game-shapes.md` (Ir per iteration and per update) and `findings/interp-f32-probes.md`
> (libm comparisons, interpreter behaviour today, generated Rust). Numbers marked *assumed* are not measured.

## 0. Verdict

**Sound as designed, with the unit bit-identical to the interpreter by construction.** A unit computes every
`f32` operation as the same instruction sequence the interpreter's `float_binary` (`src/interpreter/ops.rs`) runs:
round each operand to f32, compute in f64, round the result to f32. Values keep the interpreter's own representation:
an f64 payload tagged `F32`. That makes the unit bit-identical to the walker and the closure engine in every case,
including the cases where the interpreter differs from compiled Rust today. The unit does not need, and does not
claim, any fact about IEEE single precision, so there is no theorem to trust on the parity side. Cranelift is used
only for f64 operations the f64 units already use, plus `fdemote`/`fpromote` (cvtsd2ss/cvtss2sd on x86_64, `fcvt`
on aarch64). These are the instructions rustc emits for the interpreter's own `as f32` / `as f64`.

**Not argued, read this first:**

1. **f32 transcendentals in the interpreter are NOT bit-identical to compiled Rust, today.** The interpreter computes
   `x.sin()` on an `f32` as `(x as f64).sin() as f32` (`float_fn!` in `src/interpreter/builtins/num.rs`). Generated
   Rust calls `f32::sin`, which is glibc `sinf` on linux-gnu. On the probe host the two disagree in the last bit for
   a measurable share of inputs for every function probed: sin, cos, tan, exp, exp2, ln, log2, log10, atan, atan2,
   cbrt, sinh, tanh and pow (P1 in `findings/interp-f32-probes.md`). `powi`, `to_degrees` and `to_radians` disagree
   too, because Rust defines them with f32 operations (P2). This doc does **not** fix that. It is parity-ledger
   finding L1/L2 below. The unit design keeps transcendentals as call-outs to the interpreter's own function (stage
   F4b), so units add no new disagreement. The obstacle to fixing L1 is the platform libm. Making the interpreter call
   `sinf` would match compiled Rust on the same host. But glibc, macOS libm and the Windows CRT give different last
   bits for the same input, and so do glibc versions. So "matches compiled Rust" can only ever mean "on the same
   host", and committed golden outputs must never contain transcendental f32 bits.
2. **NaN payload and sign when both operands are NaN with different payloads.** x86 returns the first operand's
   payload. Whether the interpreter binary (LLVM may commute `fadd`/`fmul`) and a unit (Cranelift's egraph may
   canonicalize operand order) agree on which operand is first is not argued. The f64 units have the same exposure
   today. Rust itself does not promise NaN payloads for arithmetic results (RFC 3514). The oracle (section 7)
   therefore prints arithmetic NaN results as `NaN` and compares payload bits only where no arithmetic touched the NaN.
3. **Nested struct fields** (`e.pos.x` with `pos: Vec2`) are refused by units for every width (section 1, item 8).
   This doc makes the *flat* game shapes native. The nested case needs a nested row path, which belongs to
   [interp-struct-vec-loops.md](interp-struct-vec-loops.md) and is listed as a dependency, not designed here.
4. **Native single-precision registers** (stage F6) are designed only as an option. Their parity argument rests on a
   theorem (innocuous double rounding) and on ledger finding L3 being fixed first. They are not part of the plan
   until a measurement shows the carrier's conversions matter.

## 1. F0: what refuses f32 today (audit)

The root cause is one function. Everything else follows from it.

1. `ty_of` and `ty_is` in `src/interpreter/resolved/jit/kinds.rs` return "not a scalar" for `Value::Float(_, F32)`.
   So an f32 local classifies as `Opaque`, and an f32 field has type `None` in a `Shape`. A struct with an f32 field
   is never `Class::Value`, because that needs every field to be a scalar. A `Vec<f32>` has element class
   `Unknown`. Every statement that reads such a value runs in the closures, and the verdict is
   `outside the subset: N of its M statements run in the closures` (probe p10 in
   `findings/interp-e1-e6-game-shapes.md`; P3 in `findings/interp-f32-probes.md` shows both loops of a flat f32
   `Vec<struct>` program refused).
2. `literal` in `src/interpreter/resolved/jit/lower_expr.rs` refuses the `f32` integer-suffixed literal (`"f32" => return None`).
3. `cast` refuses every float target except f64 (`type_is_float(&ty) != Some(FloatTy::F64)`).
4. `coerce` refuses a declared `f32` (`Some(_) => None`).
5. `from_code` in `kinds.rs` has no case for code 102. `code` maps `F32` to 102, so a helper asked for an f32
   (`index_get`, `field_get`) would fail.
6. `row_field` in `src/interpreter/resolved/jit/rows.rs` admits only f64 and i64 fields, and `row_store_ok` in
   `lower_rows.rs` admits only those.
7. **Latent hazards that the refusal hides.** If `ty_of` admitted `F32` and nothing else changed, the unit would
   compile f32 arithmetic *without rounding*, because `flt` in `lower_expr.rs` is width-agnostic. The same holds
   for `method` (`sqrt` and the others), for `retag` of an integer literal, and for `cast` from an f32 to f64. They
   would be silently wrong, not refused. Stage F1 changes all of them together, and the mutants in section 6 pin
   each one.
8. **Width-independent refusals** that also block game code: `target` (`lower_expr.rs`) supports `v[i].f`, `s.f` and
   struct-valued locals, but not `v[i].f.g`. A `for x in v` over a `Vec<struct>` is not a loop candidate. A call
   with an element-place argument stays a call (rows a1-a4 and the probe table in the E1-E6 finding).

## 2. The reference: the interpreter's f32 today, and where it differs from compiled Rust

The walker and the closure engine share one implementation: `src/interpreter/resolved/fast.rs` calls
`ops::float_binary`. A `Value::Float(f64, FloatTy::F32)` holds an f64. The producers below normally store a value
representable in f32:

- `float_binary`: the result type is the left operand's tag, unless that is `Lit`, in which case it is the right
  operand's. For `F32` the operation rounds both operands (`a as f32 as f64`), computes in f64 and rounds the result.
  An integer literal operand is converted `i128 -> f64` first.
- `coerce` of a float to a declared f32 rounds. `cast` to f32 converts to f64 first (`*i as f64` for an integer) and
  then rounds.
- `float_fn!` and the two-argument float builtins (`src/interpreter/builtins/num.rs`) compute the f64 function and
  round with `fl`.
- Unary minus is `Value::Float(-f, t)`, with no rounding.
- Display rounds (`*f as f32`, `src/interpreter/format.rs`), and so does `to_bits` (`(x as f32).to_bits()`).

For `+ - * / sqrt`, rounding an f64 result to f32 gives the correctly rounded f32 result, because f64 has at least
2x24+2 significant bits (innocuous double rounding). `%` is exact in both widths. Comparisons of rounded operands
are exact. So the interpreter's basic f32 arithmetic agrees with compiled Rust. P3 shows a 1000-step f32 sum
matching per-operation f32 rounding.

**Parity-ledger findings (interpreter vs compiled Rust).** These are recorded here and fixed elsewhere. The unit
mirrors whatever the interpreter does when the unit lands (section 3).

| id | construct | interpreter | compiled Rust | evidence |
|---|---|---|---|---|
| L1 | `sin cos tan exp exp2 ln log2 log10 atan atan2 cbrt sinh tanh powf` on f32 | f64 function, rounded | glibc `sinf`-family | P1 |
| L2 | `powi`, `to_degrees`, `to_radians` on f32 | f64, rounded | f32 operations (square-and-multiply; multiply by an f32 constant) | P2 |
| L3 | a float or integer literal assigned to an f32 place (`y = 0.1`); `16777217_f32`; integer literal coerced to f32 | **unrounded** f64 kept under the F32 tag (`retag` in `src/interpreter/eval_stmt.rs` and `src/interpreter/resolved/compile.rs`; the `"f32"` suffix in `src/interpreter/eval_expr.rs`; `Int(Lit)` in `coerce`) | rounded at compile time (`0.1_f32`, P5) | P3: `y as f64` prints `0.1` |
| L5 | `f32::from_bits(sNaN).to_bits()` | quieted (0x7fc00001) | unchanged (0x7f800001) | P4 |
| L6 | literal-only arithmetic in an f32 context (`let x: f32 = 0.1 * 3.0`) | f64 arithmetic, rounded once at the coercion | f32 arithmetic per operation | argued from `float_binary` (Lit with Lit stays Lit); not probed |

L4 is fixed (row removed): an integer converts to `f32` with one rounding (`i128 as f32`, `u128 as f32`), never through
`f64` (`cast` in `src/interpreter/coerce.rs`; the `Int(Lit)` coercion and the `f32` suffix round once the same way).
`tests/f32_parity/f32_int_cast.wj` pins every integer width against compiled Rust. Machine code refuses `as f32` today
(`lower_expr.rs` `cast` returns `None` for a non-f64 target); the F1 stage must lower an integer to `f32` as one
`fcvt_from_sint F32` (or `fcvt_from_uint`), NOT `fcvt_from_sint F64` followed by a demote, which is the double rounding.
The unit-lowering table row for `as f32` from an integer is updated to match.

L3 is the one that matters for the unit's representation. An F32-tagged value can hold an f64 that f32 cannot
represent, and that is observable: through `as f64`, through `Value` equality and ordering (`value.rs` compares raw
f64 payloads), and through the copy of a field. The carrier design reproduces it exactly. A native-f32 design would
not (section 3.3).

## 3. The design

### 3.1 Type lattice

`Ty::Flt(FloatTy::F32)` becomes a unit scalar type. `ty_of`, `ty_is`, `bits`, `value` and `from_code(102)` admit it.
`Shape` field types, `Class::Value` struct locals, `Elem::Scalar` vector elements and map values then follow
mechanically, because they are all built from `ty_of`. Mixed `F32`/`F64` operand pairs are refused, because rustc
rejects them, so a checked program never has them. `F32` with `Lit` and `F32` with an integer literal are accepted
and follow the interpreter's rule.

### 3.2 Representation: the rounded-f64 carrier (v1)

- **Register.** An f32 value in a unit is a Cranelift `F64` SSA value (`cty` unchanged), holding exactly the f64
  payload the interpreter's `Value` would hold. Usually that is an f32-representable double. Under L3 it can be an
  unrounded one.
- **Slot encoding.** Spills, OSR entry and exit, helper arguments and results, row views and packed views all hold
  the same 64-bit f64 bits as today. `from_bits`/`to_bits` in `lower.rs` are unchanged. `kinds::bits` returns
  `f.to_bits()` for an F32 value, exactly as for F64. Nothing in `osr.rs`, `rt.rs` packed paths or `hoist.rs` changes
  except where a type code is compared.
- **Operations** (in `flt`, `method`, `cast`, `coerce`, `retag`, `literal`) mirror the interpreter instruction for
  instruction. `R(x)` stands for `fpromote(fdemote(x))`, the image of `x as f32 as f64`:
  - binary `+ - * /` with result type F32: `R(R(a) op R(b))`
  - `%`: `R(fmod(R(a), R(b)))`, with the existing `rt::fmod` helper
  - comparisons: `fcmp(cc, R(a), R(b))`, with the same condition codes as the f64 path (`Ne` is unordered-or-not-equal)
  - an integer-literal operand: `R(fcvt_from_sint F64 (i))`
  - unary minus: `fneg(a)`, unrounded, like the interpreter
- **Static elision.** `Val` gets a `rounded: bool` beside its constant field `k`. Operand rounding is skipped when the
  operand is known rounded: an F32 operation result, an `R(..)`, or a constant folded at lowering time
  (`f64const((c as f32) as f64)`). Variables and loads are not known rounded in v1, because of L3. The result
  rounding is never skipped.
- **Why the carrier and not native f32.** It is the interpreter's own computation, so parity needs no theorem and
  survives L3. It touches only the lowering of operations, not storage, OSR, views or helper signatures. The cost is
  up to 6 conversions per binary operation, fewer with elision. That is *assumed* to be a small part of an
  iteration, and it is checked by stage F1's gate.

### 3.3 Storage widths

| place | today | f32 decision |
|---|---|---|
| `Value::Float(f64, F32)` | 8-byte payload + tag | unchanged |
| `Packed::Float(Vec<f64>, F32)` (`src/interpreter/vecbody.rs`) | 8-byte elements | unchanged. Views key on type code 102 (`view_key`), so an F32 view is never used as an F64 one. A 4-byte packing is a separate memory change, not part of this design |
| row view field (`rows::Layout`) | payload at `lay.float` | same offset (the tag is a separate byte). `row_field` admits F32. `row_store_ok` admits an F32 or `Lit` value into an F32 field: the walker's `retag` keeps the F32 tag and the unrounded bits (L3), which is exactly a bits store |
| spill slot, OSR buffer | 64-bit bits | the f64 bits |
| helper signatures (`helper(.., float=true)`) | F64 in and out | unchanged; F4 helpers take and return F64 carriers |
| hoisted row (`hoist.rs`, `lay.float` for any `Flt`) | f64 | unchanged |

### 3.4 Stage F6 (optional): native f32 registers

`cty(F32) = F32`. Boundaries become `fdemote` on load and `fpromote` on store. Operations become `fadd.f32` and the
rest. Parity then rests on (a) innocuous double rounding for `+ - * / sqrt`, and (b) every F32 value reaching a unit
being f32-representable. (b) is false while L3 stands: a load would round an unrounded payload, and a copy would
change it. F6 is allowed only after L3 is fixed in the interpreter (rounding at the three producers), and only if
the F1 measurement shows the carrier's conversions cost more than *assumed*.

## 4. Hazards, one decision each

| hazard | exact in Cranelift? | interpreter today | unit decision |
|---|---|---|---|
| `+ - * /` | yes (`fadd`..`fdiv` F64, then `fdemote`/`fpromote`) | round operands, f64 op, round result | the same sequence (`R(R(a) op R(b))`) |
| comparisons, NaN operands | yes (`fcmp`, unordered semantics as the f64 path) | rounded operands; NaN compares false except `!=` | the same, with a `Lit` operand rounded first (mutant `f32litcmp`) |
| `-x` | yes (`fneg` flips the sign bit, NaN too) | `-f`, unrounded | `fneg`, unrounded |
| `%` | no `frem` in Cranelift | f64 `%` of rounded operands, rounded | `rt::fmod`, rounded around it |
| division by zero, overflow to inf | yes (no trap; `fdemote` overflows to inf like `as f32`) | IEEE inf/NaN | no guard, no exit (the f64 path has none either) |
| subnormals | yes (no FTZ/DAZ: unit and interpreter share the thread's MXCSR/FPCR, which neither sets) | kept (P4) | nothing to do; parity programs include subnormals |
| FMA contraction | Cranelift has no fusion rule; `fma` only when emitted explicitly; and the `R` between `fmul` and `fadd` makes fusion impossible in the carrier | rustc never contracts | nothing to do; a parity program whose result changes if `a*b+c` is fused pins it |
| x87 / excess precision | not used on x86_64 (SSE2) or aarch64 | none | none |
| `sqrt` | yes (`sqrt` F64) | `fl(x.sqrt())` | `R(sqrt(a))` |
| `abs floor ceil trunc` | yes (exact on any input) | `fl(f(x))` | `R(op(a))`; R is the identity on a rounded input but not on an L3 value, so it is kept |
| `round` | **no**: `nearest` is ties-to-even, Rust is half away from zero | `fl(x.round())` | `R(rt::round(a))` (the f64 path's helper; mutant `f32nearest`) |
| `min max clamp` | **no**: `fmin`/`fmax` propagate NaN and order -0 < +0; Rust's ignore NaN | `fl(x.min(y))` with `y` unrounded | F4a helper calling the same `f64::min`/`max`/`clamp`, then R (mutant `f32fmin`) |
| `signum copysign fract recip rem_euclid mul_add hypot` | partly | `fl(f64 formula)` | F4a helpers calling the shared formula |
| transcendentals, `powi powf`, `to_degrees to_radians` | no (no Cranelift op) | `fl(f64 fn)` (L1, L2) | F4b call-outs to the **same Rust function** the builtin runs; interpreter-exact, not compiled-Rust-exact |
| `as f32` from f64 / Lit | yes (`fdemote`+`fpromote` = cvtsd2ss/cvtss2sd, as rustc emits for `as`) | `f as f32 as f64` | `R(a)` |
| `as f32` from an integer | yes | one rounding, `i as f32` (L4 fixed) | `fcvt_from_sint/uint F32 (i)` then `fpromote`: single rounding, never via F64 |
| `as f64` from f32 | (retag) | payload unchanged | `Val` retagged, no instruction |
| `as iN/uN` from f32 | yes (`fcvt_to_sint_sat`/`uint_sat`: NaN to 0, saturation) | trunc, NaN to 0, clamp, on the payload | the existing f64 cast lowering on the payload; `i128`/`u128` still refused |
| literal `1.5` in f32 context | | `Lit` f64; rounded where an op meets F32 | `Lit` `f64const`; rounded at the op (folded when constant) |
| literal `16777217_f32` | | f64 of the integer, F32 tag, unrounded (L3) | `f64const(i as f64)` tagged F32, unrounded |
| `y = <Lit>` with `y: f32` (retag) | | bits kept, tag F32 (L3) | the same: `Val` relabelled, no rounding |
| `to_bits` | yes (`fdemote` + `bitcast` I32 + `uextend`) | `(x as f32).to_bits()` as u32 | F5: the same; `from_bits` refused in v1 (a static path call, not a method) |
| NaN payload through conversions | same instructions in both | cvtsd2ss/cvtss2sd quiet and truncate | identical; two-NaN operand choice not argued (section 0, item 2) |
| printing | runtime only | `Display` of `x as f32` | not in units |

## 5. Exits and guards

Float operations never trap, so f32 adds **no new exit**. The guards a unit relies on are the existing ones:

- the class and type speculation at entry (`osr::classify`): a local seen as F32 that is now F64 fails the class test
  and the loop does not enter
- the helper type check: `index_get`/`field_get` with `want = 102` against an F64 element fails into the existing
  bail
- the view key: `view_key` and `row_key` carry the type code or shape

"Bail" for floats means only those. There is no `f32bail` counter, because nothing new can bail. The new counter is
`f32units`: units compiled with at least one F32 operation. It goes in the fast-path table in
`src/interpreter/engine.rs` and the `[fast]` line in `src/interpreter/runner.rs`, beside `rowview`. Each stage also
gets a kill switch, `fast_off("f32")`, which makes F32 refuse again as it does today.

## 6. Staged plan

Every stage has: parity programs `jit_f32_<stage>*.wj` under `tests/engine_parity/`, run by
`tests/interpreter_engine_parity_test.rs` both with the default threshold and eagerly (`WJ_JIT_LOOP_HOT=0`); a
counter assertion in `tests/interpreter_fast_paths_test.rs` (`f32units` fires on them, so parity is not vacuous); the
debug-build mutants listed (`WJ_JIT_MUTANT`), each of which must change some parity program's output; and a soundness
note under `docs/design/interp-soundness/` with a row in its README.

**F0 (this document).** The refusal audit (section 1) and the ledger (section 2), with probes in
`findings/interp-f32-probes.md`. The L3-L5 fixes go to the parity ledger as separate work. Their order relative to
F1-F5 does not matter, because the unit mirrors the interpreter's formulas and those change in lockstep: the formula
functions move into `ops.rs`/`num.rs` as named functions that both engines call or mirror.

**F1: scalars.** `ty_of`/`ty_is`/`from_code`; `flt`, `literal`, `coerce`, `retag`, `cast` to f32, and the
no-argument methods `sqrt abs floor ceil trunc round`, all with the carrier rounding; `Val.rounded`.
- Mutants: `f32noround` (skip the result R), `f32endround` (round only at stores, which double-rounds a chain),
  `f32litcmp`, `f32nearest`, `f32operand` (skip operand R: observable only with an L3 value, so the program builds
  one), `f32nofold` (fold a literal unrounded).
- Gate: probe p10 (`findings/interp-e1-e6-game-shapes.md`) from 858 Ir per iteration to about the f64 figure (19.1 in
  E6) plus the conversions.
- **Stop here if** it is above 60 Ir per iteration (*assumed* budget): profile before F2.

**F2: flat struct fields.** Row views (`row_field`, `row_store_ok`), `Field`/`IndexField` helpers with code 102, F32
fields in `Class::Value` locals, and pure inlined methods over f32 structs (`Vec2::dot`).
- Mutants: `f32rowwidth` (store an F32 field through the int payload offset), `f32rowlit` (refuse to retag, which
  rounds a `Lit` store; observable via `as f64`).
- Gate: E3(a) row a6 (flat, hand-inlined, f32) from 25,257 Ir per update to the f64 twin's 258 order. **Stop if** it
  is above 2x the f64 twin.
- The nested `Vec2` rows (a1-a4) stay refused, by item 8 of section 1, until a nested row path exists.

**F3: f32 vectors.** Packed `Vec<f32>` read and store views, push-inline and hoisting. These are the generic paths
once code 102 is admitted.
- Mutants: `f32viewkey` (an F32 view answering an F64 read).
- Gate: an f32 axpy/sum loop within 1.3x of its f64 twin (*assumed* target).

**F4a: exact builtins with arguments.** `min max clamp signum copysign fract recip rem_euclid mul_add hypot`, as
helpers calling the shared num.rs formula. This needs method calls with arguments in units, which f64 lacks too, so
both widths get it. Mutant: `f32fmin` (Cranelift `fmin`).

**F4b: transcendentals.** Call-outs to the shared function. They are bit-identical to the interpreter because they are
the same machine code in the same process. Gate: every function in P1 over the oracle's inputs, unit against walker,
exact.

**F5: casts and bits.** All integer widths in both directions, `to_bits`. Mutant: `f32cast` (single rounding for
i64 as f32: it differs from the interpreter while L4 stands).

**F6: optional native registers** (section 3.4).

Ordering against the inline-callee ladder: F1 and F2 come before S2/S3. Without them, inlining saves nothing on f32
code: E3(a) a6, the hand-inlined f32 update, stays at 25.3k Ir per update.

## 7. The bit-exactness oracle

Three layers, because no single oracle covers everything:

1. **Engine parity (exact, gates every stage).** Each `jit_f32_*.wj` program prints `to_bits()` of every result as
   hex. It runs on walker, resolved, and resolved with `WJ_JIT_LOOP_HOT=0`, and the outputs must be byte-identical.
   - Inputs: ±0, ±inf, a quiet NaN, the smallest and largest subnormals, `MIN_POSITIVE`, `MAX`, neighbours one ulp
     apart, 0.1, 1/3, 2^24±1, 2^53+1 as an integer, and a block of values from an in-program integer LCG mapped with
     `f32::from_bits` (exponent-limited to finite values).
   - Operations: every op and builtin of the stage, inside a hot `while` loop so the unit runs it, and once outside
     so the walker runs it.
   - NaN results of arithmetic print as `NaN`. Bits are printed only for NaNs that no arithmetic touched (section 0,
     item 2).
2. **Compiled Rust (exact for the basic operations, on the same host).** The same programs, restricted to `+ - * / %
   sqrt abs floor ceil trunc round neg`, comparisons, `as` between floats and narrow integers, and literals rounded
   at compile time (no L3-L6 constructs). They run through the Rust backend in `tests/backend_conformance_test.rs` as
   new `tests/conformance_programs/` entries. Expectations come from an independent oracle, as that harness requires:
   Python `struct` round-trips per operation (the probe scripts' `r32`). A hand-written Rust twin goes in
   `tests/interpreter_rust_parity_test.rs`.
3. **Ledger probes (not gating).** The P1/P2 scripts, re-run whenever L1/L2 are revisited, on the host in question.

## 8. Bench side (design only)

Add game-shaped kernels whose output is exact in every runtime. They use only `+ - * / sqrt` and print a checksum of
`to_bits` values:

- `bench/interp/nbody32.wj`: nbody with `f32` fields.
- `bench/interp/ecs32.wj`: a flat f32 `Vec<struct>` update, hand-inlined in a `while`.
- `ecs32call.wj`: the same with a free function. This is the S2/S3 target.

Work units:

1. Kernels plus `bench/interp/expected/<k>.out`.
2. Twins in `bench/interp/twins/`:
   - node: `Math.fround` after every operation. It is exact by innocuous double rounding.
   - python3: a `struct.pack('f')` round trip per operation, with the emulation stated in the table's note. Mark it
     N/A if the 1x run exceeds the 10 s ceiling in `sizes.toml`.
   - lua: `string.unpack("f", string.pack("f", x))`.
3. `bench/interp/scaling/templates/<k>.{wj.tmpl,mjs,py}`, `[<k>.<label>]` sections in `bench/interp/scaling/sizes.toml`,
   regenerated `bench/interp/scaling/gen/<label>/`, `bench/interp/scaling/expected/<label>/<k>.out`.
4. Rows in `bench/interp/fallbacks.tsv` and expectations in `tests/interpreter_fallbacks_test.rs`.
5. Entries in `instr_baseline.json`/`compare.json` after the first measurement. They are measured, not typed.

## 9. Interactions

- **fnunit** ([fnunit2.md](interp-soundness/fnunit2.md)): f32 parameters enter through `read_bits` like f64 ones.
  `unit_body` is unchanged.
- **hoist** ([hoist.md](interp-soundness/hoist.md)): the hoisted offset uses `lay.float` for every float, which is
  correct for F32. The new helpers (`fmod`, `round`, F4) take no context, cannot clear rows, and belong in
  `SAFE_CALLS` like `fmod`.
- **pushlen** ([pushlen.md](interp-soundness/pushlen.md)): the kind guard compares `Packed::Float(_, w)` with the
  pushed value's tag, so F32 and F64 never mix. A `Lit` pushed into a `Vec<f32>` takes the helper, as today.
- **vecinline** ([vecinline.md](interp-soundness/vecinline.md)): view keys carry code 102. Nothing else changes.
- **vecpass retag** (branch `perf-vecpass-share`): the call-site retag deliberately does not serve `Vec<f32>`,
  because the generic coercion rounds. So F3 always sees rounded packed elements from that path. Extending the retag
  to f32 would need the rounding and is not part of this design.
- **inline-callee S1-S3** ([interp-inline-callee.md](interp-inline-callee.md)): they inherit F32 with no extra work.
  The ECS gain needs F1+F2 plus S2/S3 plus a for-in loop and nested row paths (section 1, item 8).
- **hot reload**: `Shape` carries the field type, so a field changed from f32 to f64 changes the shape and the loop
  does not enter. There are no new cache keys.
- **unaffected**: the typed scalar pipeline ([pipeline.md](interp-soundness/pipeline.md)), the int-function JIT,
  intdense ([intdense.md](interp-soundness/intdense.md)), and constdiv. They handle integers only. Lean maps
  ([maplean.md](interp-soundness/maplean.md)) move bits generically, so a `HashMap<i64, f32>` value works once
  code 102 decodes.
