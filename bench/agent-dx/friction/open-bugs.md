# Open compiler bugs in the friction corpus

Every event that `replay.wj` finds still failing on the current compiler (`open-compiler-bug`: `wj check` accepts the
program, the compiled build rejects it) has a line here with the stream that owns the fix. `replay.wj` and
`tests/agent_cost_ratchet_test.rs` fail when an open event is missing; delete a line when the replay reports the event
`fixed`. Owner streams: `closure-values` (codegen of closures and blocks used as values), `ownership` (ownership and
String inference), `adapters` (iterator adapters and collect), `widths` (integer width solver), `traits`, `std-surface`.
Programs: `events/<id>/main.wj`; minimal repros of the pilot-6 classes are in `docs/drift/pilot6-repros/`.

| Event | Owner stream | Class | First error on the current compiler |
|---|---|---|---|
| `p3-t003-B-2-c00` | ownership | owned/borrowed String inference | WJ0003: mismatched types: expected an owned `string`, found a borrowed `string` |
| `p3-t006-B-0-c00` | std-surface | char predicate argument | WJ0017: trait bound not satisfied: expected an `Fn(char)` closure, found `char` |
| `p3-t006-B-1-c00` | ownership | owned/borrowed String inference | WJ0003: mismatched types: expected an owned `string`, found a borrowed `string` |
| `p3-t006-E-0-c00` | ownership | owned/borrowed String inference | WJ0003: mismatched types: expected an owned `string`, found a borrowed `string` |
| `p3-t006-T-1-c00` | ownership | String + &String | WJ0020: cannot add `&std::string::string` to `string` |
| `p3-t013-E-2-c00` | adapters | a lazy adapter bound to a `Vec` annotation | WJ0003: mismatched types: expected `Vec<string>`, found `Map<Iter<'_, i64>, {closure@...}>` |
| `p3-t013-T-2-c00` | ownership | use after move the interpreter accepts | WJ0007: value `t` is used after it was moved |
| `p3-t018-B-0-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p3-t018-B-1-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p3-t018-E-0-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p3-t018-E-1-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p3-t018-E-2-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p3-t018-F-0-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p3-t018-F-1-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p3-t018-T-1-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p3-t018-T-2-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p3-t034-T-0-c00` | traits | trait used as a type | WJ0034: a trait cannot be used as a plain type |
| `p3-t050-F-0-c00` | std-surface | `len` on a split iterator | WJ0015: no method `len` found for `std::str::Split<'a, P>` |
| `p4-t003-K-2-c00` | widths | integer width | WJ0003: mismatched types: expected `i32`, found `i64` |
| `p4-t013-K-0-c00` | ownership | use after move the interpreter accepts | WJ0007: value `t` is used after it was moved |
| `p4-t018-K-0-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p4-t018-K-1-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p4-t018-K-2-c00` | widths | integer width | WJ0003: mismatched types: expected `i32`, found `i64` |
| `p5-t104-U-0-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p5-t104-U-1-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p5-t104-U-2-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p5-t108-U-2-c00` | closure-values | block value | WJ0003: mismatched types: expected `void`, found `string` |
| `p6-t104-U-0-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p6-t104-U-1-c00` | ownership | borrow inference at a map call | WJ0003: mismatched types: expected `&_`, found `string` |
| `p6-t104-U-2-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p6-t104-V2-0-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p6-t104-V2-0-c01` | adapters | a lazy adapter bound to a `Vec` annotation | WJ0003: mismatched types: expected `Vec<string>`, found `Map<Iter<'_, _>, {closure@...}>` |
| `p6-t104-V2-1-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p6-t104-V2-2-c00` | ownership | reference to an owned map (`unwrap_or(HashMap::new())`) | WJ0003: mismatched types: expected `&HashMap<string, i64>`, found `HashMap<_, _>` |
| `p6-t104-V2-2-c01` | ownership | String against str from an indexed Vec<string> | WJ0003: mismatched types: expected `string`, found `str` |
| `p6-t104-V2-2-c03` | ownership | String against str from an indexed Vec<string> | WJ0003: mismatched types: expected `string`, found `str` |
| `p6-t104-V2-2-c05` | ownership | String against str from an indexed Vec<string> | WJ0003: mismatched types: expected `string`, found `str` |
| `p6-t104-V2-2-c06` | ownership | String against str from an indexed Vec<string> | WJ0003: mismatched types: expected `string`, found `str` |
| `p6-t104-V2-2-c08` | ownership | String against str from an indexed Vec<string> | WJ0003: mismatched types: expected `string`, found `str` |
| `p6par-t104-U-0-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p6par-t104-U-1-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p6par-t104-U-2-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p6par-t104-V2-0-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
| `p6par-t104-V2-1-c00` | widths | integer width | WJ0003: mismatched types: expected `uint`, found `int` |
| `p6par-t104-V2-2-c00` | closure-values | a block-bodied closure or `if`/`else` whose value is used (comparator returning `Ordering`, block value) | WJ0003: mismatched types: expected `Ordering`, found `void` |
