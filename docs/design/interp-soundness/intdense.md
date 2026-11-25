---
status: current
last_verified: 5457283f4156
---
# perf-kmap-intdense: soundness note (written before the change; repeated in the commit body)

Scope: `src/interpreter/kmap.rs` gets a fourth store, `IntDense`, for hashed maps (`HashMap`, never `BTreeMap`, never sets)
with integer keys and `i64`-valued integer values. `Vec<i64>` values with the key implicit in the index (base 0) plus a
presence bitmap `Vec<u64>`: 8 bytes and one bit per entry against 64 (Dense) or about 97 (Table). Context: the hashmap-flat report on pc-results.

## What the store claims about the map
* every key is `Value::Int` of ONE int type `kt`, with index `0..=u32::MAX` (exactly the old `index()` range);
* every value is `Value::Int` of ONE type `vt`, `vt` in {I64, Isize, Lit} (all 64-bit signed), and the integer fits an `i64`
  (`Lit` is held i128 in the interpreter; a literal-width value past i64 is refused where it arrives);
* bit `i` is set iff key `i` is present; absent slots hold 0 (cleared on remove, so a stale word never reads as present);
* `len` = popcount of the bitmap; `vals.len()` is the array's reach (as the old `slots.len()`).
There is no base offset: the old Dense array had none (key = slot), so promotion/growth rules are identical and no
key-range logic is new. A negative key can never be held: policy = fall back (demote), never offset.

## Promotion (once per map lifetime at most)
Only for `hashed` maps, and only when `int_off` is false, at two places, both already O(n) once per doubling:
1. `condense()` (Table -> array when len is a power of two >= 8 and keys fit): builds the IntDense directly from the table
   entries (skipping the 64-byte array entirely) when `IntDense::from_entries` accepts them, else the old Dense;
2. `promote()` after an insert into a `Dense` array that makes `len` a power of two >= 8 (a map filled 0..n never was a table).
`from_entries` accepts iff all keys are `Value::Int` of one type with a slot, and all values are `Value::Int` of one
`typed_ok` type that fits i64. Anything else (a u64/i32/bool/float/string/Unit value, mixed Lit and I64 values, two key
types) refuses and the map stays on the old store. `Tree`-born maps (`tabulate`) are not promoted at construction.

## Demotion (to `Dense`, same slots, same keys, same values; then `int_off = true`: never promoted again, so no flip-flop)
On the first operation the typed array cannot serve exactly: `get_mut` (a `&mut Value` into the map: place projections,
`entry().or_insert`, `m[k] += x`, `iter_mut`, `values_mut`); an insert whose value is not `Value::Int(_, vt)`/does not fit
i64, or whose NEW key is not `Int(_, kt)`/needs growth the dense rule refuses (then the old code spreads it to a Table
or tree as before); `remove` of a key without a slot that is not hash-exact (a float: the old code scans); `untabulate`,
`spread`. An insert of an EQUAL key present already keeps the stored key (BTreeMap rule) so only the value type matters.
`clear()` keeps the typed array (empty, kt/vt retained): a later foreign insert demotes an empty array in O(1).
Cost bound: promotion O(n) per doubling, demotion O(n) once per map: total amortized O(1) per insert, no cliff.

## Observable behaviour that must be identical, and why it is
* **Iteration order.** Old Dense iterated its slots in index order = ascending key order; Table sorted by key; Tree sorted.
  The language documents HashMap order as unspecified (ADR-0021 D6; `--hash-seed` permutes it at `ordered()` by sorting
  on a hash of the key, independent of the store), but this repo's observable default is ascending key order for every
  store. `IntDense` iterates bitmap bits ascending (forward and `next_back`), so it is the same sequence. Verified by the
  differential unit test (forward, reversed, `ordered(0)`, `ordered(5)` compared with the non-typed store at every 53rd step)
  and by order-sensitive digests in engine_parity/kmap_intdense.wj whose lines equal the old binary's output.
* **Absent vs zero.** A stored 0 sets its bit; `get`/`contains_key` test the bit, never the value. `remove` clears the bit
  and zeroes the word and decrements `len` only if the bit was set (`take`). Tested in unit and parity tests.
* **Value identity.** Reads rebuild `Value::Int(x as i128, vt)` and keys `Value::Int(i, kt)`: bit-identical to what was
  stored because the type is fixed and the integer fits. `Cow` is returned by the accessors (borrowed from old stores, owned
  here); no caller depends on a `&Value` address (the walkers `heap.walk` and `moves` only read).
* **Equal keys of another int type.** The old Dense matched keys by index alone, ignoring the key's `IntTy`; so does `find`.
* **Overwrite** returns the old value as `Value::Int(old, vt)`; `set_int` overwrites in place (present slot, or an absent slot
  inside the reach with the key type `kt`), otherwise falls through to `insert`.
* **remove/clear/len/extend/retain/pop_first/first_key_value/range/union...** all go through `insert`/`remove_entry`/`iter`/
  `ordered`; `retain` builds a fresh `empty_like()` map (a Dense, promoted again by the normal rule).
* **Copy-on-write.** `KMap` is `Rc<KMap>`; every writer goes through `Rc::make_mut`, which clones the whole `KMap`
  (derive(Clone) deep-copies the two Vecs). A clone therefore never shares mutable storage. Tested: clone then mutate in
  both directions (unit + parity). `int_off` is cloned along with the map.
* **Lean JIT helpers.** `rt::map_get` re-reads the cell on every call (nothing cached across calls, as in SOUNDNESS-maplean);
  `KMap::typed_get` answers `Some(Some((bits, vt)))`/`Some(None)` for an IntDense, `None` for the other stores (old path), and
  the value is still checked against the unit's wanted width by `lean_bits` (a width mismatch FAILS, nothing changed, as
  before). `rt::map_insert` -> `set_int` -> IntDense fast path; a foreign value type falls to `insert` which demotes.

## Stays on the old store (promotion refused), by design
i32/u8/u16/u32/u64/usize/i128/u128 values, bool/float/string/struct/Option values, sets (Unit values), BTreeMaps, string/float/
char keys, negative or > u32::MAX keys, maps whose int keys come in two int types, maps that have ever been demoted.

## "Drop the duplicated key" and "skip the Table detour"
* Duplicated key: free for the typed store (implicit key); the old Dense keeps its 64-byte entry (a second layout change
  there would need a different `Option<Value>` layout and slot API; not done, not free).
* Table detour: partly free: `condense()` builds the typed array straight from the table (no 64-byte Dense in between), so
  the 64 MB Dense Vec at 100x is never built. The Table itself (grown to ~100 MB before the first power-of-two check at
  len 524 288) is still built: avoiding it needs a different growth rule for a map that starts empty; left for later.

## Counters
`fast_paths` has `mapleantyped` (lean lookups answered by an IntDense). Promote/demote counts would need process-global
state in `kmap.rs` (the architecture `static_state` ratchet); not added: promotion is asserted through `mapleantyped > 0`
on the kernel and through `KMap::store_kind()` in the unit tests.

## Tests
unit (kmap.rs): `typed_array_answers_as_the_previous_store` (24 seeded random sequences, typed vs `int_off` map: every
insert/set_int/remove/get/get_key_value/get_int/clone/get_mut/clear result and iteration, forward/back/ordered), promotion
thresholds and refusals, demotion cases + stickiness, zero/absent/remove-bit/bitmap word boundary/both-end iteration,
COW + clear, Lit width. engine_parity: kmap_intdense.wj, kmap_intdense_sweep.wj (outputs equal the pre-change binary's).
fast_paths: kernel and sweep assert `mapleantyped`. Real mutations per hunk: see the intdense report on pc-results.
