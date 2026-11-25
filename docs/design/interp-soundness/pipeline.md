---
status: current
last_verified: 5457283f4156
---
# Typed scalar pipeline: soundness argument

Scope: `drain_iter` (the materialising consumers: sum, product, count, collect, ...) over a chain
`Map|Filter` ... over a source of packed `Int` scalars (`Packed::Int` Vec body, or a bounded integer `Range`).
Floats, bools, `take`/`skip`/`zip`/`enumerate`/... nodes, `find`/`any`/`all`/`for` (they pull with `next`, not drain)
stay on the boxed path (dropped from this step: no argument written for them).

## Design: speculate, and on ANY doubt restore and run the boxed path from the start
The pipeline is a pure function of (source items, closure programs, captured integer values). It either
  (a) drains the whole chain and returns the results, setting the iterator state exactly as the boxed drain leaves it, or
  (b) declines/bails, restoring the one piece of state it touched (the source cursor) to its value on entry, and the
      unchanged boxed `next` loop runs from scratch.
So the only things to argue are: the result of (a) equals the boxed result, and (b) leaves no trace.

## Guards (all checked before the first element; any failure = decline, boxed path)
G1 engine_resolved, no tparams, `drop_types` empty, no `hot`, no limits: exactly the conditions of `call_compiled_closure`
   (so a closure runs compiled-or-walker the same way); and the `fast_off("scalarpipe")` switch.
G2 every node from the top down to the source is `Map` or `Filter`, with `peeked` empty (no lookahead state to honour);
   the source is `Buf` over `Packed::Int`, over an unpacked body whose items are all `Int` of the first item's width
   (each item checked as it is read; a mismatch or a non-i64 value is a bail), or a bounded `Range` (both ends fit i64, count <= MATERIALISE_LIMIT).
G3 each stage function is a `Callable::Closure` whose AST body (compared with the compiled form: same
   literal typing via `literal_width`/`int_literal`, same name resolution) is built only from: the single plain
   parameter `x` (`|x|`), integer literals, captured integer locals, `+ - * / %`, the six comparisons,
   `*`/`&` of an int (identity on ints), and nothing else. That grammar contains no call, method call, assignment,
   block, macro, closure, `await`/spawn, or `&&`/`||`/`!`: it cannot run user code, mutate an environment slot,
   print, panic other than by the arithmetic below, or observe anything but its inputs. A body that is a constant
   sub-expression pair (`2 * 3`) is declined: the compiled engine lowers those through `typed.rs`, which this
   argument does not cover.
G4 captured names: resolved as the closure compiler resolves them (the LAST environment binding of that name); the
   slot must hold a plain `Value::Int` that fits i64 and is not borrowed (`try_borrow`); anything else (a `Ref`,
   a float, a struct) declines. The captured values are read ONCE at prepare time: sound because no stage can run
   code that writes an environment slot (G3), and nothing else runs between the first and last element.
G5 types are resolved statically per stage: element type `t` from the source tag, literal types from the same
   facts the compiler uses, `unify` for arithmetic. Any 128-bit width, a non-Int/Bool stage result, a Map whose
   program is boolean or a Filter whose program is not boolean declines.
G6 `depth_check()` is called once; if it errors the pipeline declines (the boxed call raises it at the same point).

## Per-element equivalence
The boxed closure body evaluates the same tree through `fast::binary`/`int_op`, left operand then right operand,
then the operator. `int_op` has one fast branch (both operands fit i64, result checked-op, `ty.fits(r)`) and
otherwise falls to `int_cmp` (comparisons) or `ops::int_binary` (overflow, zero divisor, `i64::MIN / -1`, width
overflow: the panics and their messages and locations). The typed evaluator implements exactly the fast branch and
returns `None` (bail) in every other case: overflow, `fits` failure, zero divisor, `MIN / -1`. Where it succeeds it
computes the identical integer and the identical result type (`unify` is static). Where it does not, bail (b) is
taken, so the error message, the `mark`ed location, and the element at which it is raised are produced by the
unchanged boxed code. Elements before the failing one were evaluated twice, which is invisible because the stages are
pure (G3) and the source is read-only during the drain.

Order of effects: the boxed drain pulls element by element, running map1, filter, map2 for each element before the next
one. The typed loop does the same per element; stages are pure so the only visible order is the order of FAILURES,
and failures are reproduced by the boxed rerun in the boxed order. A failing `Filter` predicate that returns a
non-bool in the boxed path (truthy => false) cannot occur: G5 only admits boolean-typed predicates.

## State after success (a)
* source cursor: `Buf.pos = end` / `Range.cur = end`, as the boxed loop leaves it after the final `None`;
* `it.cur` (the location `mark` last set, which a later error without its own mark would report): set to the last
  location the boxed path would have marked (statically: the last `lead_loc` of the last executed stage that has one);
* closure caches, counters (`fast.*`), `rstack`, scope, depth: boxed calls restore these on return, so the boxed drain
  leaves them as it found them (only perf counters differ; they are not program-observable);
* result: `Vec<i64>` + element type, turned into `Value::Int(x, ty)` exactly as boxed maps produce.
`sum`'s own overflow check (`add_ints`) is unchanged and still runs after the whole chain (the boxed `sum` also drains
first, then adds), so a closure panic at element k surfaces before a sum overflow at element j<k, as before.

## Why bail restores correctly
Between entry and bail the pipeline wrote nothing but a local Vec and (at success only) the cursor and `cur`. On
bail nothing was written. The mutation tests (below in the commit) replace the bail by a wrong result and by a
non-restoring path to show the tests catch each.

## Kept boxed (and why)
Floats (float_binary/NaN/`mark` details not argued), bool maps, any call/side-effect/capture-mutation closure,
non-scalar elements, `take`/`take_while`/`skip`/`zip`/`enumerate`/`chain`/`flat_map`..., `peek`ed nodes,
early-exit consumers, 128-bit and u64/usize elements (not packed), debug/limits/hot runs.

## sum (commit 2)
The boxed `sum` drains first (all closures run, in order), then folds `add_ints` (int_binary Add per item, width
`unify(acc, item)`). The typed `sum` adds in i64 while the chain produces, checking after each addition that the running
total fits the result width `unify(declared-or-item width, item width)`. Any failure (i64 overflow, width overflow, a
closure bail) is a bail of the whole pipeline: it wrote nothing, the boxed path then runs all closures first and adds
second, so the order of failures (closure panic at item k beats sum overflow at item j < k) and the overflow message are
the boxed ones. Declined: float or non-integer type hints, two different concrete widths (declared vs item), 128-bit.
An empty result returns the boxed seeds. Partial sums that leave the width (negatives) bail too, where boxed also fails.
Stage function pointers: `x aop k` / `(x aop k) cop c` run exactly `Op::ev`'s checked i64 operation and width check; the
operator is a const generic; non-matching shapes use the `Op` tree.
