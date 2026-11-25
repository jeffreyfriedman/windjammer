---
status: current
last_verified: 57637b4f6b6d
---
# Soundness: retagging a local literal vector at a `Vec<f64>` / `Vec<iN>` call (`retag_arg`, resolved/expr.rs)

## What was slow
Binding a parameter coerces the argument to the declared type. `coerce` (src/interpreter/coerce.rs) rebuilds a `Vec`
whose FIRST element is an untyped literal (`needs_coerce`): a new `Vec<Value>` with every element coerced, O(n) per call,
and the caller's vector is untouched, so the next call rebuilds it again. A vector filled with `v.push(1.0)` (or `v.push(k)` for an untyped integer `k`) holds untyped
float (integer) literals (`FloatTy::Lit`; packed bodies carry the tag once: `Packed::Float(_, Lit)`), so every call of a function
taking it by value paid the rebuild, while a vector of typed floats, or one held in a struct field (the field is not coerced
at the call), paid nothing. It is neither a deep copy of an `Rc` target nor a copy-on-write: the body is never cloned by the
call, it is rebuilt by the coercion.

## The change
When a module-function call site passes a plain local (unboxed: `X::Slot`; boxed: the walker cell that holds the vector
itself, not a reference to another place) to a parameter declared `Vec<f64>`/`Vec<float>` or `Vec<iN>`/`Vec<uN>`/`Vec<int>`
(`vec_retag`), and the callee does not mutate that parameter, the site first runs `retag_arg`: if the local's body holds
untyped literals of that family, the LOCAL's slot is replaced by a NEW body (`VecBody::retagged`,
src/interpreter/vecbody.rs) with the same values tagged with the formal's element type. Binding then finds no literal to
coerce, and every later call with the same local is free. Counter: `vecretag` (one per retagged local, not per call).
Not served, left to the generic coercion: `Vec<f32>` (rounds), a nested vector, a field or element place as the argument,
a mutated parameter (its argument is a place: `place::arg`), a vector with any element that is not a literal of the
family or the formal's own width.

## Why it is observationally the generic path
1. Values. A `Lit` float coerced to `f64` is `Float(f, F64)` with the same `f`; a `Lit` integer coerced to width `t` is
   `Int(n, t)` with the same `n`: the coercion the callee would have applied (`coerce`, src/interpreter/coerce.rs) is exactly
   the one applied to the local. Integer RANGE: the retag is made only when `t.fits(n)` for EVERY `Lit` element; otherwise
   it declines and the generic coercion runs and raises its own `literal out of range` error (a `u8` formal with 300, a
   negative into an unsigned, an `i8` with 200, an `i16` with 40000: `vec_retag_int_range_*.wj`), so nothing wraps and the
   error is the same one. The literal `-1` for a 64-bit unsigned formal (which the generic path wraps) is outside `fits`
   and also stays on the generic path. `Vec<f32>` is NOT served (it rounds); a `Lit` float in an integer vector, or the
   reverse, is not served.
2. The caller's later reads. The only difference visible in the caller is the tag of elements read back (`F64` where it
   was `Lit`). An untyped literal adopts the other operand's width in arithmetic (`ops.rs`: `Lit` takes the other side's
   type); an `F64` element keeps `f64`. A program that passes the local to a `Vec<f64>` parameter has typed that local
   `Vec<f64>`, so its elements ARE `f64` in the compiled program; mixing them with `f32` operands is a type error there.
   The parity program `vec_retag.wj` pins the visible arithmetic.
3. Aliases. The slot gets a NEW body (never a tag change in place): every other holder of the old `Rc` (a struct field, a
   clone, a closure capture) keeps the old body, so no unique-body fast path (`Rc::strong_count == 1` checks of the hoist
   views, pending-length push, vec-inline read views, intdense/map-lean helpers) sees a changed buffer or tag. By-value
   semantics of the call are unchanged: the callee gets what coercion would have given it, a value-equal vector.
4. Machine-code views. A read/store/push view is keyed by slot and element type and holds the old buffer's address.
   The retag runs inside the closure engine's call expression; machine code reaches closure code only through `run_stmt`,
   which clears all views and resolved cells first ([vecinline.md](vecinline.md), invalidation set 2), and a replaced slot
   value is the same event as any assignment `v = other`. A pending push length is flushed before every helper call
   ([pushlen.md](pushlen.md)). The old body is dropped only after the slot is overwritten and no view survives that.
5. Parameters the callee mutates are not served: those arguments are places (`place::arg`), not `X::Slot`.
6. Stores of untyped float literals into an `f64` packing (`Packed::put`, vecbody.rs). A retagged (or otherwise typed) float
   body is `Packed::Float(_, F64)`. A later `v.push(1.5)` / `v[i] = 1.5` carries an untyped literal; storing it used to
   decline (tags differ), which unpacked the body into `Vec<Value>` (O(n), and the packed fast paths were lost for good).
   Now an untyped float literal stored into an `f64` packing is stored as `f64`. Soundness: the vector holds `f64`
   elements, so it is a `Vec<f64>` and the literal is one of its elements: same rule as item 1 (a `Lit` adopts the other
   operand's type; the value is unchanged). An `f32` packing, a packing that is itself `Lit`, and a typed value of another
   width still decline and unpack, as before. Visible difference: reading the element back gives `F64` instead of `Lit`
   (item 2). The caller-pushes-between-calls pattern (`peek(v); v.push(2.5)`) therefore neither unpacks nor needs a
   second retag. Integers need no such rule: measured on the base build, an integer literal pushed into a typed integer
   packing already stays packed (`vec_shape_adopt.wj`: 6 of its 8 pushes), so only the float store was changed.
   Not served: an annotated EMPTY vec (`let mut v: Vec<float> = []`) still holds `Lit` after literal pushes (the empty
   body has no element type), so it is retagged once at its first call (one O(n) copy of the packed buffer, no per-call
   work; `vec_shape_a.wj`).

   Companion change (cost only, no semantics): the "does this argument need coercion" test looked at the first element
   through `Deref` (`t.first()`), which materializes a packed body's whole `Vec<Value>` view (O(n)) and is dropped again by
   every push; with the packing kept across pushes that made shape D O(n) per call (2.6M Ir per call at 100k elements).
   `VecBody::first_is` asks the packed scalar directly (`needs_coerce`, and the two `Vec` arms of `coerce`), so a call
   neither unpacks nor builds the view. Unit test `needs_coerce_does_not_unpack_a_packed_vector` (coerce.rs).

## Tests
`tests/engine_parity/vec_shape_{a..g,adopt}.wj` (the downstream shapes: annotated empty vec, two hops, returned vec, push between calls, constructors, element stores, struct field; exact counts in `the_downstream_vector_shapes_retag_at_most_once`), `tests/engine_parity/vec_retag.wj`, `vec_retag_int.wj` (a thousand calls: one retag), `vec_retag_int_range_*.wj` (walker vs resolved; read-only callee with the caller pushing between calls, a mutating
callee, return, struct store, `f(v, v)`, recursion, method, `Vec<f32>`, nested, closure capture); counter `vecretag` in
`tests/interpreter_fast_paths_test.rs`; kernel `bench/interp/vecpass.wj`.
