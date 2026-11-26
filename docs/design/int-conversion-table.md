---
status: accepted
last_verified: 69b4b500a
---
# Implicit integer width conversions: one table

The declared width wins (ADR-0015, ADR-0039 principle 5). Four sites declare a width: an annotated binding
(`let wide: u64 = v.len()`), an argument to a declared parameter, a return, and a struct field initializer.
All four ask the same question, answered by one table: `src/type_inference/int_conversion_table.rs`
(`conversion(from, to)`). Before this table, `is_safe_implicit_cast` (solver), `declared_int_target_name` /
`maybe_cast_usize_to_int_target` (codegen, only `int`/`i32`) and the literal suffix code each decided alone, so
`let size: int = v.len()` and `let small: i32 = v.len()` converted while `let wide: u64 = v.len()` did not.

## The table (64-bit targets; `usize`/`isize` are 64 bits)

| Class | Meaning | Rows |
|---|---|---|
| Same | nothing to do | `T -> T` |
| Widen | every value fits; implicit, emitted as `as` | signed to wider signed; unsigned to wider unsigned; unsigned to strictly wider signed; includes `usize <-> u64`, `isize <-> i64`, `usize -> u128/i128`, `u64 -> usize` |
| LossyAs | the existing ergonomics rule, kept exactly; implicit, emitted as `as` | `i32->u64, i32->u8, i16->u8, i32<->usize, u32<->i32, usize->u32, i32->u32, i64->i32, i64<->u64, i64<->usize` |
| Error | not implicit | everything else: signed to unsigned of any other pair, any narrowing not listed above |

Integers never convert to floats. At a declared float slot an integer literal is emitted as a float literal
(`w = 3` stores `3.0_f32`); a non-literal integer expression stays a type diagnostic (WJ0003 family).

## Literals

A literal has no width of its own: the declared slot decides it. A literal above `u64::MAX` lexes as a 128-bit
payload (`Literal::Wide`); a declared `u128`/`i128` slot types it, and no declared 128-bit slot leaves it to
rustc, which rejects it. A literal at a declared 128-bit slot goes through the same table (`literals::literal_for_declared_width`): `let h: u128 = 18446744073709551615` is `_u128`, never `_u64`.

## Where it lives

* `int_conversion_table.rs`: the table (`IntWidth`, `Conversion`, `conversion`, `is_implicit`).
* `int_implicit_casts.rs::is_safe_implicit_cast`: the solver's view, now a thin wrapper over the table.
* `codegen/rust/usize_expression_type_inference.rs`: `declared_int_target_name` (the declared widths) and
  `usize_store_cast_suffix` (the cast text) consult the table; every `usize` store site calls them.
* `codegen/rust/literals.rs::literal_for_declared_width`: the literal side.

## No drift

`tests/int_conversion_table_rustc_test.rs` enumerates all 144 pairs of {i8..i128, u8..u128, isize, usize}, generates
one rustc program per class and runs it: every Widen pair must be lossless by `TryFrom` on MIN and MAX, every
LossyAs pair must lose a value, and the table's class must equal the class computed independently from ranges.
