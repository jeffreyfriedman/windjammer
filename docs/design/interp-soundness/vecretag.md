---
status: current
last_verified: 57637b4f6b6d
---
# Soundness: retagging a local float vector at a `Vec<f64>` call (`retag_arg`, resolved/expr.rs)

## What was slow
Binding a parameter coerces the argument to the declared type. `coerce` (src/interpreter/coerce.rs) rebuilds a `Vec`
whose FIRST element is an untyped literal (`needs_coerce`): a new `Vec<Value>` with every element coerced, O(n) per call,
and the caller's vector is untouched, so the next call rebuilds it again. A vector filled with `v.push(1.0)` holds untyped
float literals (`FloatTy::Lit`; packed bodies carry the tag once: `Packed::Float(_, Lit)`), so every call of a function
taking it by value paid the rebuild, while a vector of typed floats, or one held in a struct field (the field is not coerced
at the call), paid nothing. It is neither a deep copy of an `Rc` target nor a copy-on-write: the body is never cloned by the
call, it is rebuilt by the coercion.

## The change
When a module-function call site passes a plain unboxed local (`X::Slot`) to a parameter declared `Vec<f64>`/`Vec<float>`,
the site first runs `retag_arg`: if the local's body is a packed `Float(_, Lit)` vector, or an unpacked one whose elements are
all `Float(_, Lit | F64)` led by a `Lit`, the LOCAL's slot is replaced by a new `Rc` body with the same bits tagged `F64`
(`VecBody::retagged_f64`, src/interpreter/vecbody.rs). Binding then finds no literal to coerce. Counter: `vecretag`.

## Why it is observationally the generic path
1. Values. A `Lit` float coerced to `f64` is `Float(f, F64)` with the same `f`: the coercion the callee would have applied
   is exactly the one applied to the local. The `f32` target is NOT served (it rounds), nor is any other element type, nor
   a nested vector, nor an argument that is not a plain local.
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
6. Pushing a literal to a retagged vector no longer extends the packing (`Packed::put` wants the exact tag), so the body
   unpacks once: a cost, not a semantic change.

## Tests
`tests/engine_parity/vec_retag.wj` (walker vs resolved; read-only callee with the caller pushing between calls, a mutating
callee, return, struct store, `f(v, v)`, recursion, method, `Vec<f32>`, nested, closure capture); counter `vecretag` in
`tests/interpreter_fast_paths_test.rs`; kernel `bench/interp/vecpass.wj`.
