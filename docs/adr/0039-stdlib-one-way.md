---
status: proposed
last_verified: af4e69ade
---
# ADR-0039: One correct way: the standard library surface

- **Status:** Proposed. Owner answers (2026-11-16): decision A, keep lazy iterators with `.collect()` as the one way to a
  list; decision D, remove `unwrap`; decision B, open (the analysis below is written for that decision).
- **Date:** 2026-11-16
- **Related:** [ADR-0014](0014-surface-syntax-rust-isms.md) (macros and Rust-isms; this ADR finishes its migration),
  [ADR-0015](0015-language-consistency-proposals.md) section 1 (`int` default), [ADR-0018](0018-expressiveness-phase-2.md)
  (`T?`, `??`, enumerable methods), [ADR-0028](0028-usize-class-in-the-numeric-solver.md) and
  [ADR-0029](0029-int-at-the-surface.md) (integer widths; `usize` stays), [ADR-0030](0030-interpreter-rejects-what-rustc-rejects.md)
  (interpreter == compiled), [ADR-0037](0037-agent-cost-levers.md) (agent cost, measured offline), ADR-0002 (explicit
  `let mut`, unchanged). Design notes settled here: `docs/design/split-returns-list.md`, `docs/design/list-max-min.md`.
- **Machine-readable surface:** [`docs/std_surface.toml`](../std_surface.toml) (254 canonical rows, 62 removal rows,
  two of them pending decision B).
- **Inventory:** `wj run --interpret scripts/wj/std_inventory.wj --eco <ecosystem checkout>` writes
  `docs/generated/std_removal_inventory.tsv`; `--widths` measures the casts behind decision B (counts below at `b71288999`).

## Context

The owner's direction (2026-11-15): do the stdlib fixes and the redesign now, remove deprecations, no backward
compatibility, and make the idiomatic way to do each thing obvious: one correct way. Windjammer is pre-1.0, so a
breaking change costs one `wj fix` run today and a migration guide for every user after the public release.

What agents hit (pilot data; failures that were not harness denials):

| Failure class | Where it comes from today |
|---|---|
| forgot `.collect()` after `split` (2 of 8 failures; 12 of 40 trials in pilot 7) | `split`, `lines`, `chars` return a lazy iterator; the checker catches indexing it but not every list-only use, and the interpreter accepts `.len()` on it |
| hand-rolled `abs`, `min`, `max`, `contains`, `parse`, char predicates, `[x; n]` | the forms exist but are spread across methods, free functions in `std::strings` / `std::math`, and ASCII/Unicode twins |
| `.len() as int` / `as usize` casts (321 length casts and 776 `as usize` across the corpora) | `len()` is `usize` when compiled; the agent sheet itself says `let n: int = v.len() as int` |
| `parse().unwrap()` flagged W0002 with no obvious clean path | `unwrap` is an error-level lint, `expect` too; `?` / `??` exist but are not presented as the path |
| list `max()` / `min()` lowering to `Ord::max(self, other)` (E0061) | the same name means "two numbers" and "largest element"; the lowering guessed |
| `items.map(..).collect()` lowering to `items.as_ref().iter()` (E0282) | the lowering leaves the collect target to rustc instead of emitting the type the checker knows |
| interpreter accepting what the compiled build rejects | engine-specific aliases: 69 interpreter methods are not in the documented surface |
| deprecated forms still accepted | `vec![x; n]`, `println!`, `{}` templates, `String` warn or are lowered, not refused |
| Rust aliases next to Windjammer forms | `iter`, `unwrap`, `push_back`, `to_owned`, `contains_key`, `strings.to_upper` next to `for x in xs`, `??`, `to_uppercase` |

Today there are six hand-maintained descriptions of the built-in surface: `agent_index/builtins.txt`,
`tests/ledgers/agent_index_unlisted_methods.txt`, `src/stdlib_surface/known_methods.rs`,
`src/stdlib_surface/enumerable.rs` (with `std/enumerable.wj`), `std/rust_core.wji`, and the interpreter registry
(`src/interpreter/builtins/*.rs`). They agree only where a gate forces them to.

## Principles

1. **One canonical form per task.** For every common task there is exactly one spelling. A second spelling is removed,
   not deprecated: it becomes WJ0180 with the canonical form as a machine-applicable fix, or it is simply absent.
2. **One vocabulary across types.** The same operation has the same name on every type that has it: `len`, `contains`,
   `first`, `last`, `get`, `is_empty`, `to_uppercase`, `is_digit`, `cmp`, `min`, `max`. A name never means two different
   operations on types an agent cannot tell apart at a glance.
3. **Sequences are lazy; `.collect()` makes a list.** Every built-in that yields several values (`split`, `lines`,
   `chars`, `keys`, `values`, set operations, every adapter) returns a lazy `Iter`. `.collect()` is the one way to a
   list. Using an `Iter` where a list is needed is a hard error with a one-edit fix, identical in `wj check`, the
   interpreter and `wj build`.
4. **Absence is a value, handled with `?`, `??` or `match`.** No method silently crashes on absence; `expect("why")` is the
   one explicit crash, for invariants.
5. **Integer widths are explicit where the program declares them.** `usize` and every other width stay first-class
   declared types. Which type a length or a position has when the program declares nothing is decision B (open).
6. **No Rust spelling survives for compatibility.** A Rust name stays only where it is also the best Windjammer name
   (`push`, `pop`, `map`, `filter`, `split`, `collect`, `rev`, `trim`, `parse`, `entry().or_insert()`).
7. **Every canonical row is documented, discoverable, tested and type-checked, identically in both engines.** Each row of
   `docs/std_surface.toml` has a `wj docs` entry (generated), an interpreter implementation, a lowering that rustc accepts
   (a row in the per-type lowering table gate), and a verified example run interpreted and compiled. A removed form is
   rejected by the shared checker before either engine runs (ADR-0030), so the interpreter can never accept it.

## Decisions

Each decision gives the options, one recommendation, its strongest counter-argument, what it breaks (counts from the
inventory below; "code" is std + tests + examples + golden + bench + scripts + ecosystem `.wj`), and a confidence.

### A. `split` and every multi-value method: lazy iterators (owner: decided 2026-11-16)

`split`, `splitn`, `rsplit`, `lines`, `split_whitespace`, `chars`, `char_indices`, `bytes`, `matches`, `keys`, `values`,
the set operations and every adapter return a lazy `Iter`, as in Rust. `for part in s.split(",")` never allocates;
`let parts = s.split(",").collect()` is a list. The cost of this choice, the forgotten `.collect()`, is designed out by
decision H's diagnostic: a hard error whose machine-applicable fix inserts `.collect()` (or `.count()`) at the exact place,
reported the same way by `wj check`, `wj run --interpret` and `wj build`. Of the 1,040 `split` / `lines` /
`split_whitespace` / `chars` calls in the corpora, 437 are `for` loops and 186 end in `.collect()` today.

Breaks: nothing beyond decision H's redundant spellings. Confidence: high (owner decision).

### B. The integer type of `len()`, counts and positions (open: owner decision needed)

**In plain terms.** Windjammer has two integer types that matter here. `int` is a signed 64-bit number on every
platform. `usize` (Rust's "machine-size unsigned integer") can never be negative and is as wide as a memory address: 64
bits on desktops and servers, 32 bits on the web (wasm32). Rust gives lengths and positions the type `usize`; Windjammer
gives every number a program writes the type `int`. Where the two meet, Rust demands a conversion, and that is where the
casts come from.

**What is the same either way.** Speed. On a 64-bit machine both are one 64-bit register; `x as usize` compiles to no
instruction at all, and the bounds check on `xs[i]` is the same compare. On wasm32 converting a 64-bit `int` index to a
32-bit `usize` is one wrap instruction, and wasm has native 64-bit arithmetic. No option below is faster or slower at
run time in any way a benchmark would notice.

**What differs.** Meaning, in three places.
- Negative values. `xs.len() - 1` on an empty list is `-1` as an `int`, and an index of `-1` panics with Windjammer's
  out-of-range message. As a `usize` the same subtraction underflows: a panic with overflow checks on, a huge number
  without them. Loops such as `while i >= 0` or `for i in 0..n - 1` are natural with `int` and traps with `usize`.
- Mixing. `int + usize` is a type error, so every place where a length meets a number the program declared (`count: int`
  in a struct, `fn f(n: int)`, `let n: int = ...`) needs a cast in today's compiled code.
- The web. `usize` is 32 bits on wasm32, so a `usize` value wraps at about four billion on the web and not natively; an
  `int` behaves identically on every target.

**Why the casts appear (measured, `std_inventory.wj --widths`).** Programs declare `int` (the default every agent and
example uses) and then meet `.len()`, or use an `int` as an index. In code: 321 casts of a length (about 300 to `int` or
`i64`, 21 to `i32`/`u32`), 674 `as usize` inside an index or slice, 21 in a std length argument, 776 `as usize` in all;
59 `let x: int = ...len()` and 102 `let x: usize` annotations. The agent-dx reference solutions alone carry 30 length
casts and 80 `as usize`.

**What the compiler already infers.** ADR-0028 (accepted, in progress) gives the width solver an index class: an
integer the program leaves undeclared becomes `usize` when it meets `len()` or an index
(`let mut i = 0; while i < xs.len() { xs[i] }` needs no cast), and a declared width wins, with the `len()` result
converted to it (`let n: int = xs.len()` is accepted). ADR-0029 (proposed) moves that decision into one type checker. So the solver already removes
the casts for undeclared integers; the casts that remain are where a declared `int` meets a length or an index. How many
of the 1,097 are strictly needed today can only be counted by a compiled sweep with them deleted (stream S5, step 1:
one build of the golden set and the examples).

**The options.**

| Option | In practice | Casts it removes (of 321 length + 776 `as usize`) | Agent sheet |
|---|---|---|---|
| **B1** lengths are ordinary numbers (surface `int`, compiler-inserted conversions) | `len()`, `count()`, positions and `enumerate` indices are `int`; `xs[i]` and every length parameter accept `int` or any declared width; the compiled code converts at the boundary | about 300 length casts + 674 index casts + 21 argument casts, about 995; left: 21 narrowing casts to `i32`/`u32` and about 80 casts into a declared `usize` | the `len()` bullet is deleted |
| **B2** lengths stay machine-size, indexes convert (Rust's `usize`, inference, checked conversion at index sites only) | `len()` is `usize`; undeclared integers that meet it become `usize` (ADR-0028); `xs[i]` with an `int` converts automatically | the 674 index casts; the 321 length casts stay wherever a declared `int` meets a length | keeps a rule: "`len()` is `usize`; to keep it in an `int`, write `as int`" |
| **B3** status quo | as today | none (beyond the ones ADR-0028 already makes unnecessary) | unchanged |

Golden set: B1 removes all 30 length casts and 61 of the 80 `as usize` (54 index, 7 argument); B2 removes the 54 index
casts; B3 none.

**The "performance pass" idea** (let the compiler pick `usize` or `int` per value, for speed). It would decide from
run-time cost, and run-time cost is identical for both types on 64-bit targets and differs by one free instruction on
wasm32, so the pass would have nothing to decide from. The one place width matters for performance is memory: a large
`Vec<u32>` of indices is half the size of a `Vec<int>`. That is a layout choice that also changes which values fit, so
it must stay the program's declaration (`Vec<u32>`), which every option keeps. What such a pass would add is one more
place that decides integer widths, which is the class of code (the W2-W12 deciders) ADR-0029 is deleting. It adds
nothing here.

**Recommendation: B1**, confidence medium-high: high that it removes the most casts and gives the same meaning on every
target (no underflow traps, no 32-bit surprise on the web), medium on the solver work (S5). What would change it: the
S5 step-1 sweep showing that today's solver already accepts nearly every program with its casts deleted (then B2 costs
less for the same result), or a wasm benchmark where 64-bit index arithmetic costs more than 5%.

Breaks under B1: R030, R031 (counted as `pending` until the owner decides). Under B2: the index-site part of R031.

### C. `min` / `max` and numbers

Options for sequences. (1) `xs.max() -> T?` (lowered `.iter().max().cloned()` on a list). (2) Drop it; use `fold`.
Options for numbers. (a) Methods `a.max(b)`, `x.abs()` (Rust). (b) Free functions `max(a, b)`, `abs(x)` (Python, Go, Swift).

**Recommendation: (1) + (a).** Operations are methods on the value, everywhere: `x.abs()`, `x.sqrt()`, `x.pow(n)`,
`a.max(b)`, `xs.max() ?? 0`. The arity tells the two `max` apart, and so does the receiver's declared type, which the
lowering now reads instead of guessing (the E0061 came from the guess). `std::math` keeps constants (`PI`, `E`, `TAU`,
`SQRT_2`) only; its free functions (`math.abs_i64`, `math.max_f64`, `math.sqrt`) are removed (R070). `powi`/`powf`
become `pow` (the exponent's type picks the lowering). Floats get `cmp` with total order (NaN last), so
`xs.sort_by(|a, b| a.cmp(b))` and `xs.sort()` work on floats and the `partial_cmp(..).unwrap()` idiom disappears (R072).
An agent that writes `max(a, b)` or `abs(x)` gets WJ0180 R073 with the method form as the fix.

Counter-argument: `max(a, b)` is symmetric and is what most agents write first. Answer: a free-function family would
duplicate every numeric method or leave an arbitrary split; the did-you-mean diagnostic turns the first-draft spelling
into a one-edit fix.

Breaks: R070 (34), R071 (5), R072 (5); R073 is a did-you-mean, not a removal. Confidence: high.

### D. Parsing, validation and the Option / Result idioms (owner: decided 2026-11-16)

| Situation | Write |
|---|---|
| the caller should handle the failure | `let n = s.parse::<int>()?` (any function returning `Result` or `T?`, including `fn main() -> Result<(), string>`) |
| a default is right | `let n = s.parse::<int>() ?? 0` (`??` works on `T?` and on `Result`, dropping the error) |
| both cases need code | `match s.parse::<int>() { Ok(n) => .., Err(e) => .. }` or `if let Some(x) = o { .. }` |
| absence is a bug (an invariant) | `o.expect("config was validated above")` |

`unwrap()` is removed (R040). `expect(why)` is canonical and no longer linted, so W0002 retires. `unwrap_or(d)`,
`unwrap_or_else(f)`, `unwrap_or_default()`, `map_or`, `is_some_and` and their Result twins become `??` compositions
(R041-R047): `o.map(f) ?? d`, `o.map(p) ?? false`. The Option surface is `is_some`, `is_none`, `map`, `and_then`,
`filter`, `or`, `ok_or`, `take`, `replace`, `zip`, `flatten`, `expect`; Result is `is_ok`, `is_err`, `ok`, `err`, `map`,
`map_err`, `and_then`, `expect`. `parse::<T>()` keeps its name and returns `Result<T, string>`. `wj fix` writes `?` where
the function returns Option/Result, otherwise `.expect("<expr> is present")`.

Breaks: R040 (314 in code), R041 (1,207, auto), R042-R048 (35). Confidence: high (owner decision).

### E. Char predicates and string naming

Options. (1) Keep Rust's two families (`is_alphabetic` / `is_ascii_alphabetic`, `is_numeric` / `is_ascii_digit` /
`is_digit(radix)`). (2) One Unicode-aware family, ASCII by composition.

**Recommendation: (2).** The char surface is `is_alphabetic`, `is_alphanumeric`, `is_digit(radix = 10)`,
`is_whitespace`, `is_uppercase`, `is_lowercase`, `is_punctuation`, `is_control`, `is_ascii`, `to_uppercase`,
`to_lowercase`, `to_digit(radix = 10)`. ASCII-only tests are `c.is_ascii() && c.is_alphabetic()`. Bytes (`u8`) answer
the same names. Strings: `to_uppercase`, `to_lowercase`, `eq_ignore_case`, `trim*`, `strip_prefix`/`strip_suffix`
(returning `string?`), `find` (returning `int?`) (byte offsets throughout: `len()`, `find`, `s[a..b]` all count bytes; characters are
`s.chars()`, their number `s.chars().count()`). Appending is `s += t` (string or char); `push_str`, string `push`,
`insert_str`, `substring`, `char_count`, `to_owned`, `as_str`, `as_bytes` go. The `std::strings` module, whose free
functions duplicate the methods under other names (`strings.to_upper`, and `strings.split` returning a list where the
method is lazy), is deleted (R058).

Counter-argument: `is_ascii_digit` and `to_ascii_uppercase` are what Rust programmers and agents know, and the Unicode
forms change behaviour on non-ASCII input. Answer: the change is confined to non-ASCII letters, where the Unicode answer
is the intended one in nearly every program; 20 call sites in code use the ASCII twins.

Breaks: R050-R058 (4,086 in code, 3,518 of them `strings.` calls, 1,766 in the ecosystem), R060-R063 (21).
Confidence: high.

### F. Filled lists and construction

`[a, b]`, `[x; n]` and `[]` build lists; `{k: v}` builds a map; `HashMap::new()`, `HashSet::new()`, `VecDeque::new()`
build the empty ones that have no literal; a set or a map from a sequence is `.collect()` into an annotated target
(`let seen: HashSet<int> = xs.collect()`). `Vec::new()` (1,888 in code) becomes `[]` (R008): the empty-list literal
already infers its element type from later use. `vec![..]` (R001, 180) and `Vec::repeat` / `fill` go.
`Vec::with_capacity(n)` stays as the one performance hint. Confidence: high.

### G. Maps and sets

`m[k]` reads (panics when missing), `m[k] = v` writes, `m.get(k) -> V?`, `m.insert(k, v) -> V?`, `m.remove(k) -> V?`,
`m.contains(k)` (the same name as list, set and string; R090 replaces `contains_key`, 86 in code), `keys()` and
`values()` are lazy (`m.keys().sorted()` is a sorted list), `for (k, v) in m` iterates, and `m.entry(k).or_insert(v)` is
the one get-or-insert form (`*m.entry(w).or_insert(0) += 1`). Arguments are values: `m[&k]` and `xs.contains(&x)` become
`m[k]`, `xs.contains(x)` (R086, 89). Ordered maps and sets add `first`, `last`, `range` with the list names (R093).
Sets have `insert`, `remove`, `contains`, `union`, `intersection`, `difference`, `symmetric_difference`, `is_subset`,
`is_superset`, `is_disjoint`, `retain`, `extend`; the set operations are lazy like every other multi-value method.

Counter-argument: `contains_key` is unambiguous on a map whose values could be confused with keys. Answer: `contains` on
a map has one sensible meaning (keys, as in Python's `in` and Kotlin's `contains`), and one membership name across every
collection is worth more. Confidence: high (contains), medium (keeping `entry().or_insert()` over a new
`m.update(k, default, f)`; the entry form works compiled today and is a single row).

### H. Iteration, adapters and `.collect()`

`for x in xs` iterates any sequence; `for (i, x) in xs.enumerate()` and `for (k, v) in m` destructure. Adapters start at
the collection (`xs.map(f)`, never `xs.iter().map(f)`) and return a lazy `Iter`: `map`, `filter`, `filter_map`,
`flat_map`, `flatten`, `enumerate`, `zip`, `chain`, `take`, `skip`, `take_while`, `skip_while`, `step_by`, `scan`, `rev`,
`peekable`. Consumers end a chain: `collect`, `count`, `sum`, `product`, `fold`, `min`, `max`, `min_by_key`,
`max_by_key`, `any`, `all`, `find`, `position`, `nth`, `last`, `join`. The enumerable methods of ADR-0018 that build a
collection (`sorted*`, `unique*`, `group_by`, `count_by`, `tally`, `partition`, `chunk_by`) return it directly.

**`.collect()` is the one way to a list,** and its target follows one rule: the type declared where the value goes
(a `let` annotation, a parameter, a return type, a field), else `Vec<T>`. `let words = s.split(" ").collect()` is a
`Vec<string>`; `let seen: HashSet<int> = xs.collect()`, `let s: string = cs.rev().collect()`,
`let r: Result<Vec<int>, string> = parts.map(|p| p.parse::<int>()).collect()`. The Rust lowering always emits
`collect::<T>()` with the type the checker resolved, so rustc never has to infer a collect target (the E0282 class ends).

**Redundant spellings, decided for lazy semantics.** `iter()`, `iter_mut()`, `into_iter()` go (adapters start at the
collection; R010). `cloned()`, `copied()` go (elements are values; the lowering adds the copy; R012). `count()` stays:
it is the consumer for an `Iter`, `len()` the size of a collection, and `.len()` on an `Iter` is the diagnostic below
with `.count()` as its fix. `rev()` stays (lazy reverse; `reverse()` reverses a list in place). `peekable()` stays for
`peek()`. `to_list()` goes (R024, it is `collect()`), `sum::<T>()` loses its turbofish (R017), `for_each` becomes a
`for` loop (R015), `by_ref` and `fuse` go (R016).

**The "iterator used as a list" diagnostic (WJ0035, broadened).** The checker knows every expression's static type, so
it reports an `Iter` that reaches a place only a list can fill:

| Use of an `Iter` | Machine-applicable fix |
|---|---|
| indexed or sliced: `parts[0]`, `parts[1..]` | `.collect()` |
| `.len()` / `.is_empty()` | `.count()` / `.collect()` |
| `.first()` | `.next()` |
| `.contains(x)`, `.get(i)`, `.sort()`, `.push(..)` or any other list-only method | `.collect()` |
| passed to a `Vec` parameter, returned from a function declared `-> Vec<T>`, stored in a `Vec` field, compared with a list, printed with `${it:?}` | `.collect()` |
| a bound `Iter` used twice (the first use consumes it) | `.collect()` on the `let` |

Placement: when the `Iter` is a `let` binding without an annotation, the fix goes on the initializer
(`let parts = s.split(",").collect()`), so every later use works; otherwise it goes at the exact expression before the
use (`s.split(",").collect()[1]`). The message prints the corrected line.

One check, three entry points: it lives in the shared front end (`src/check/semantic/checker.rs`, which already reports
WJ0035 for indexing a few std iterators), and `wj check`, `wj run --interpret` and `wj build` all run it before anything
executes or compiles, so the three report the same code, span and fix. The interpreter's run-time "an iterator cannot be
indexed" error becomes unreachable and stays only as an internal assertion. WJ0015's iterator cases defer to WJ0035.

Zero extra turns: `wj check --fix` and `wj run --fix` (new flags) apply every machine-applicable WJ0035 and WJ0180 edit
and run the result, and the agent sheet tells agents to run with `--fix`. Without the flag the cost is one edit that the
message spells out.

Breaks: R010, R012, R015-R024 (165 in code). Confidence: high on the surface; high on the diagnostic (it extends an
existing check with type facts the checker already has).

### I. Collection type names (owner veto point)

Options. (1) Keep `Vec<T>`, `HashMap<K, V>`, `HashSet<T>`, `VecDeque<T>`, `BTreeMap`, `BTreeSet` (SPEC-TYP-006, the agent
sheet, every corpus). (2) Rename to `List<T>`, `Map<K, V>`, `Set<T>` (ADR-0014 phase 2, proposed; `std/enumerable.wj`
already spells `List` and `Map`).

**Recommendation: (1)**, with `T?` as the one spelling of an optional type (R007: `Option<T>` only for nesting, 741 in
code, already rewritten by `wj fix`). The Rust names are what every agent writes first and what the whole corpus uses;
renaming them would turn a correct first draft into an error in nearly every program. `std/enumerable.wj` is rewritten
to `Vec` / `HashMap`, and prose calls a `Vec` a list.

Counter-argument: `List`/`Map`/`Set` read better, match the prose, and finish ADR-0014's direction. Answer: they are
better names but not a better outcome; a rename is cheap now and can still be taken by the owner, at the price of an
error on most agents' first draft. Confidence: medium.

### J. The deprecation endgame

Every deprecation ends now. The ADR-0014 forms and the W-lints that policed Rust spellings become one diagnostic,
WJ0180 "removed std form", driven by the `removed` table: its message names the row (`R040`), the replacement, and,
for `fix = "auto"` / `"typed"` rows, a machine-applicable edit. The parser stops lowering `println!`, `vec![]`,
`format!` and `{}` templates to the canonical nodes; it only recognises them to report WJ0180.

| Code | Today | After |
|---|---|---|
| W0002 explicit unwrap/expect | error lint | retired: `unwrap` is R040 (WJ0180); `expect` is canonical |
| W0003 explicit `.iter()` | error lint | retired into R010 |
| W0006 literal `.to_string()` | error lint | retired into R006 |
| W0007 macro spelling | warning, still lowered | retired into R001/R002, refused |
| W0008 `{}` template | warning, still converted | retired into R003, refused |
| W0010 `String`/`&str` | error lint | retired into R004 |
| W0012 redundant `.as_str()` | warning | retired into R051 |
| W0004, W0005, W0009, W0011, W0013-W0017 | unchanged | unchanged (ownership, style and crate-surface lints, not std forms) |

A retired code stays in the catalog as "retired: see WJ0180" so `wj explain W0007` still answers.

## Single source of truth and its gates

`docs/std_surface.toml` is the one hand-edited description. Each consumer is generated from it or checked against it
(`tests/std_surface_test.rs`, one test per gate; each gate gets a planted-mutation fixture proving it can fail):

| Gate | Checks | Replaces |
|---|---|---|
| G1 index | `agent_index/builtins.txt` is byte-identical to `wj agent-index --from docs/std_surface.toml` | hand edits of builtins.txt |
| G2 interpreter | every `(type, method)` the interpreter registers is a canonical row or carries `internal = true` in the registry; every canonical row is registered | `tests/ledgers/agent_index_unlisted_methods.txt` (deleted when empty) and `tests/agent_index_builtins_coverage_test.rs` (kept, reading the toml) |
| G3 lowering | every canonical row has a row in the per-type lowering table (the model is `tests/list_method_lowering_typechecks_test.rs`) whose generated Rust type-checks; no `KNOWN_GAPS` | per-method spot fixes |
| G4 typing | every row's `lower` names a method `std/rust_core.wji` declares, and the row's Windjammer signature matches the checker's | drift between surface and core types |
| G5 removed | every `removed` row with `action = "error"` has a WJ0180 oracle case under `tests/diagnostics_oracle/check/removed/` whose fix, applied, type-checks | per-lint goldens |
| G6 inventory | `scripts/wj/std_inventory.wj --check`: no count in `docs/generated/std_removal_inventory.tsv` rises and every known method name is canonical or removed; counts outside `docs/adr/` and the oracle fixtures reach 0 at the end of the migration and the gate then requires 0 (`pending` rows excepted until decided) | nothing (new) |
| G7 examples | every canonical row is exercised by an `examples/verified` program or a `tests/spec` case run walker, resolved and compiled with identical output | `wj docs` rows without evidence |
| G8 iterator as list | one oracle case per row of decision H's table under `tests/diagnostics_oracle/check/iter_as_list/`; `wj check`, `wj run --interpret` and `wj build` give the same code, span and fix, and the fixed program runs with identical output in both engines | the run-time WJ0035 in the interpreter |

Shrink-only rules: `methods` may grow only together with its G3 row, G7 example and a summary line; `removed` rows are
permanent (they are the hint table); the inventory ledger may only fall. `known_methods.rs` keeps the external and
user-type names it needs for WJ0015, minus every removed name.

## Canonical surface (snapshot)

Generated by `wj run --interpret scripts/wj/std_inventory.wj --surface`; the toml is authoritative.

| type | method | signature | notes |
|---|---|---|---|
| intrinsic | `println` | `println(msg: string)` | print a line to stdout; interpolate with "${x}", "${x:.2}", "${x:?}", "${x:>5}" |
| intrinsic | `print` | `print(msg: string)` | print without newline |
| intrinsic | `stderr.println` | `stderr.println(msg: string)` | print a line to stderr |
| intrinsic | `stderr.print` | `stderr.print(msg: string)` | print to stderr without newline |
| intrinsic | `panic` | `panic(msg: string)` | abort with message, exit status 101 |
| intrinsic | `assert` | `assert(cond: bool, msg: string = "")` | panic if cond is false |
| intrinsic | `assert_eq` | `assert_eq(a, b, msg: string = "")` | panic unless a == b |
| intrinsic | `assert_ne` | `assert_ne(a, b, msg: string = "")` | panic if a == b |
| intrinsic | `debug` | `debug(x) -> T` | print "[file:line:col] expr = value" to stderr and return x |
| intrinsic | `todo` | `todo()` | placeholder that panics with "not yet implemented" |
| intrinsic | `unreachable` | `unreachable(msg: string = "")` | panic: this path cannot run |
| literal | `[a, b]` | `[a, b, c]` | a list (Vec<T>); [] is the empty list |
| literal | `[x; n]` | `[x; n: int]` | a list of n copies of x, e.g. [0; 10] |
| literal | `{k: v}` | `{k1: v1, k2: v2}` | a map literal (HashMap<K, V>); the empty map is HashMap::new() |
| literal | `a..b` | `a..b, a..=b` | an integer range: for i in 0..n; (0..n).map(f) is an Iter |
| operator | `??` | `x ?? default` | the value inside an Option (Some) or Result (Ok), else default; default is evaluated only when needed |
| operator | `?` | `expr?` | return early with the None / Err from the enclosing function (main may return Result<(), E>) |
| operator | `[i]` | `xs[i: int], m[k], s[a..b]` | index a list (panics out of range), a map (panics when the key is missing), slice a list or string |
| operator | `+=` | `s += t` | append a string or a char to a `let mut` string; also numbers |
| seq | `len` | `xs.len() -> int` | number of elements of a collection (an Iter has count()) |
| seq | `is_empty` | `xs.is_empty() -> bool` | true if no elements |
| seq | `contains` | `xs.contains(x) -> bool` | membership test, e.g. [1, 2].contains(2) |
| seq | `count` | `it.count() -> int` | number of elements an Iter yields (consumes it), e.g. s.split(",").count() |
| seq | `map` | `xs.map(\|x\| expr) -> Iter<U>` | transform each element |
| seq | `filter` | `xs.filter(\|x\| cond) -> Iter<T>` | keep elements where cond is true |
| seq | `filter_map` | `xs.filter_map(\|x\| option) -> Iter<U>` | map and keep the Some values |
| seq | `flat_map` | `xs.flat_map(\|x\| list) -> Iter<U>` | map then flatten |
| seq | `flatten` | `xss.flatten() -> Iter<T>` | join a list of lists (or drop the None of a list of options) |
| seq | `enumerate` | `xs.enumerate() -> Iter<(int, T)>` | pairs (index, element); `for (i, x) in xs.enumerate()` |
| seq | `zip` | `xs.zip(ys) -> Iter<(T, U)>` | pairs of elements, up to the shorter length |
| seq | `chain` | `xs.chain(ys) -> Iter<T>` | concatenate |
| seq | `take` | `xs.take(n: int) -> Iter<T>` | first n elements |
| seq | `skip` | `xs.skip(n: int) -> Iter<T>` | all but the first n elements |
| seq | `take_while` | `xs.take_while(\|x\| cond) -> Iter<T>` | leading elements while cond holds |
| seq | `skip_while` | `xs.skip_while(\|x\| cond) -> Iter<T>` | drop leading elements while cond holds |
| seq | `step_by` | `xs.step_by(n: int) -> Iter<T>` | every nth element |
| seq | `scan` | `xs.scan(init, \|acc, x\| expr) -> Iter<A>` | running accumulation, one result per element |
| seq | `rev` | `xs.rev() -> Iter<T>` | reverse order (in place on a list: reverse()) |
| seq | `sorted` | `xs.sorted() -> Vec<T>` | a sorted list (in place: sort()) |
| seq | `sorted_by` | `xs.sorted_by(\|a, b\| a.cmp(b)) -> Vec<T>` | a copy sorted with a comparator |
| seq | `sorted_by_key` | `xs.sorted_by_key(\|x\| key) -> Vec<T>` | a sorted list by key (stable) |
| seq | `sorted_by_key_desc` | `xs.sorted_by_key_desc(\|x\| key) -> Vec<T>` | a copy sorted by key, largest first (stable) |
| seq | `unique` | `xs.unique() -> Vec<T>` | first occurrence of each distinct element, in order |
| seq | `unique_by` | `xs.unique_by(\|x\| key) -> Vec<T>` | first element of each distinct key |
| seq | `chunk_by` | `xs.chunk_by(\|x\| key) -> Vec<Vec<T>>` | maximal runs of consecutive elements with equal keys |
| seq | `intersperse` | `xs.intersperse(sep) -> Vec<T>` | sep between consecutive elements |
| seq | `partition` | `xs.partition(\|x\| cond) -> (Vec<T>, Vec<T>)` | (matching, rest), each in order |
| seq | `group_by` | `xs.group_by(\|x\| key) -> HashMap<K, Vec<T>>` | elements grouped by key, order kept within a group |
| seq | `count_by` | `xs.count_by(\|x\| key) -> HashMap<K, int>` | number of elements per key |
| seq | `tally` | `xs.tally() -> HashMap<T, int>` | number of occurrences of each distinct element |
| seq | `fold` | `xs.fold(init, \|acc, x\| expr) -> A` | reduce to one value |
| seq | `sum` | `xs.sum() -> T` | sum of the elements (0 when empty) |
| seq | `sum_by` | `xs.sum_by(\|x\| n) -> N` | sum of f(x) |
| seq | `product` | `xs.product() -> T` | product of the elements (1 when empty) |
| seq | `min` | `xs.min() -> T?` | smallest element, None when empty: `xs.min() ?? 0` |
| seq | `max` | `xs.max() -> T?` | largest element, None when empty: `xs.max() ?? 0` |
| seq | `min_by_key` | `xs.min_by_key(\|x\| key) -> T?` | element with the smallest key |
| seq | `max_by_key` | `xs.max_by_key(\|x\| key) -> T?` | element with the largest key |
| seq | `min_by` | `xs.min_by(\|a, b\| a.cmp(b)) -> T?` | smallest element by a comparator |
| seq | `max_by` | `xs.max_by(\|a, b\| a.cmp(b)) -> T?` | largest element by a comparator |
| seq | `any` | `xs.any(\|x\| cond) -> bool` | true if any element matches |
| seq | `all` | `xs.all(\|x\| cond) -> bool` | true if every element matches |
| seq | `find` | `xs.find(\|x\| cond) -> T?` | first matching element |
| seq | `position` | `xs.position(\|x\| cond) -> int?` | index of the first match |
| seq | `index_of` | `xs.index_of(x) -> int?` | index of the first element equal to x |
| seq | `first` | `xs.first() -> T?` | first element |
| seq | `last` | `xs.last() -> T?` | last element |
| seq | `join` | `xs.join(sep: string) -> string` | join strings with a separator, e.g. words.join(" ") |
| seq | `collect` | `it.collect() -> Vec<T>` | gather into a list; the annotated target decides another collection: let s: HashSet<int> = it.collect() (also HashMap from pairs, string from chars or strings, Result<Vec<T>, E> from results) |
| seq | `peekable` | `it.peekable() -> Iter<T>` | an Iter that can also peek() (tokenizers) |
| seq | `nth` | `it.nth(i: int) -> T?` | the element at position i of an Iter (on a list: get(i)) |
| Vec | `push` | `xs.push(x)` | append an element |
| Vec | `pop` | `xs.pop() -> T?` | remove and return the last element |
| Vec | `insert` | `xs.insert(i: int, x)` | insert x at index i |
| Vec | `remove` | `xs.remove(i: int) -> T` | remove and return the element at i |
| Vec | `swap_remove` | `xs.swap_remove(i: int) -> T` | remove i in O(1) by moving the last element into its place |
| Vec | `swap` | `xs.swap(i: int, j: int)` | swap two elements |
| Vec | `clear` | `xs.clear()` | remove every element |
| Vec | `truncate` | `xs.truncate(n: int)` | keep the first n elements |
| Vec | `resize` | `xs.resize(n: int, x)` | grow with copies of x or shrink to n |
| Vec | `extend` | `xs.extend(ys)` | append every element of another sequence |
| Vec | `retain` | `xs.retain(\|x\| keep)` | keep elements where the closure is true |
| Vec | `dedup` | `xs.dedup()` | remove consecutive duplicates |
| Vec | `sort` | `xs.sort()` | sort ascending in place (floats: total order, NaN last) |
| Vec | `sort_by` | `xs.sort_by(\|a, b\| a.cmp(b))` | sort with a comparator returning Ordering |
| Vec | `sort_by_key` | `xs.sort_by_key(\|x\| key)` | stable sort by key |
| Vec | `sort_by_key_desc` | `xs.sort_by_key_desc(\|x\| key)` | stable sort by key, largest first |
| Vec | `reverse` | `xs.reverse()` | reverse in place (lazily: rev()) |
| Vec | `rotate_left` | `xs.rotate_left(n: int)` | rotate elements left |
| Vec | `rotate_right` | `xs.rotate_right(n: int)` | rotate elements right |
| Vec | `drain` | `xs.drain(a..b) -> Vec<T>` | remove a range and return it |
| Vec | `split_off` | `xs.split_off(at: int) -> Vec<T>` | cut the list at an index; returns the tail |
| Vec | `get` | `xs.get(i: int) -> T?` | bounds-checked access |
| Vec | `get_mut` | `xs.get_mut(i: int) -> T?` | bounds-checked access for changing the element in place (`if let Some(p) = xs.get_mut(i) { p.x += 1 }`) |
| Vec | `split_at` | `xs.split_at(i: int) -> (Vec<T>, Vec<T>)` | two halves at an index |
| Vec | `starts_with` | `xs.starts_with(prefix) -> bool` | prefix test |
| Vec | `ends_with` | `xs.ends_with(suffix) -> bool` | suffix test |
| Vec | `binary_search` | `xs.binary_search(x) -> Result<int, int>` | search a sorted list: Ok(index) or Err(insertion point) |
| Vec | `windows` | `xs.windows(n: int) -> Vec<Vec<T>>` | sliding windows of size n |
| Vec | `chunks` | `xs.chunks(n: int) -> Vec<Vec<T>>` | consecutive chunks of size n (the last may be shorter) |
| Vec | `concat` | `xss.concat() -> Vec<T>` | join a list of lists |
| Vec | `with_capacity` | `Vec::with_capacity(n: int) -> Vec<T>` | an empty list with room for n elements (performance only) |
| VecDeque | `push` | `q.push(x)` | add at the back |
| VecDeque | `pop` | `q.pop() -> T?` | remove from the back |
| VecDeque | `push_front` | `q.push_front(x)` | add at the front |
| VecDeque | `pop_front` | `q.pop_front() -> T?` | remove from the front (queue order: push + pop_front) |
| VecDeque | `new` | `VecDeque::new() -> VecDeque<T>` | an empty double-ended queue |
| Iter | `next` | `it.next() -> T?` | the next element |
| Iter | `peek` | `it.peek() -> T?` | the next element without consuming it (after peekable()) |
| Iter | `next_if` | `it.next_if(\|x\| cond) -> T?` | consume the next element only when cond holds |
| Iter | `cycle` | `it.cycle() -> Iter<T>` | repeat forever (end the chain with take(n)) |
| string | `len` | `s.len() -> int` | length in bytes (characters: s.chars().count()) |
| string | `is_empty` | `s.is_empty() -> bool` | true if length is 0 |
| string | `contains` | `s.contains(pat) -> bool` | substring (or char) test |
| string | `starts_with` | `s.starts_with(pat) -> bool` | prefix test |
| string | `ends_with` | `s.ends_with(pat) -> bool` | suffix test |
| string | `find` | `s.find(pat) -> int?` | byte index of the first match |
| string | `rfind` | `s.rfind(pat) -> int?` | byte index of the last match |
| string | `replace` | `s.replace(from, to) -> string` | replace every occurrence |
| string | `replacen` | `s.replacen(from, to, n: int) -> string` | replace the first n occurrences |
| string | `trim` | `s.trim() -> string` | strip surrounding whitespace |
| string | `trim_start` | `s.trim_start() -> string` | strip leading whitespace |
| string | `trim_end` | `s.trim_end() -> string` | strip trailing whitespace |
| string | `trim_matches` | `s.trim_matches(c: char) -> string` | strip a char from both ends, repeatedly |
| string | `trim_start_matches` | `s.trim_start_matches(pat) -> string` | strip a prefix repeatedly |
| string | `trim_end_matches` | `s.trim_end_matches(pat) -> string` | strip a suffix repeatedly |
| string | `strip_prefix` | `s.strip_prefix(p) -> string?` | the rest after a prefix, None when absent |
| string | `strip_suffix` | `s.strip_suffix(p) -> string?` | the rest before a suffix, None when absent |
| string | `split` | `s.split(sep) -> Iter<string>` | the pieces between separators, lazily: for p in s.split(",") { .. }; a list: let parts = s.split(",").collect() |
| string | `splitn` | `s.splitn(n: int, sep) -> Iter<string>` | at most n pieces |
| string | `split_once` | `s.split_once(sep) -> (string, string)?` | split at the first separator |
| string | `rsplit_once` | `s.rsplit_once(sep) -> (string, string)?` | split at the last separator |
| string | `split_whitespace` | `s.split_whitespace() -> Iter<string>` | the words between runs of whitespace |
| string | `lines` | `s.lines() -> Iter<string>` | the lines, without line endings |
| string | `chars` | `s.chars() -> Iter<char>` | the characters |
| string | `char_indices` | `s.char_indices() -> Iter<(int, char)>` | (byte index, char) pairs |
| string | `bytes` | `s.bytes() -> Iter<u8>` | the UTF-8 bytes |
| string | `matches` | `s.matches(pat) -> Iter<string>` | the non-overlapping matches |
| string | `rsplit` | `s.rsplit(sep) -> Iter<string>` | the pieces from the right |
| string | `to_uppercase` | `s.to_uppercase() -> string` | upper-case copy (Unicode) |
| string | `to_lowercase` | `s.to_lowercase() -> string` | lower-case copy (Unicode) |
| string | `eq_ignore_case` | `s.eq_ignore_case(t) -> bool` | case-insensitive equality |
| string | `repeat` | `s.repeat(n: int) -> string` | s repeated n times |
| string | `parse` | `s.parse::<T>() -> Result<T, string>` | parse a number or bool: `s.parse::<int>()?` or `s.parse::<int>() ?? 0` |
| string | `pop` | `s.pop() -> char?` | remove the last char |
| string | `truncate` | `s.truncate(n: int)` | keep the first n bytes |
| string | `clear` | `s.clear()` | empty the string |
| char | `is_alphabetic` | `c.is_alphabetic() -> bool` | letter (Unicode); ASCII only: c.is_ascii() && c.is_alphabetic() |
| char | `is_alphanumeric` | `c.is_alphanumeric() -> bool` | letter or digit |
| char | `is_digit` | `c.is_digit(radix: int = 10) -> bool` | digit in a radix: '7'.is_digit(), 'f'.is_digit(16) |
| char | `is_whitespace` | `c.is_whitespace() -> bool` | space, tab, newline, ... |
| char | `is_uppercase` | `c.is_uppercase() -> bool` | upper-case letter |
| char | `is_lowercase` | `c.is_lowercase() -> bool` | lower-case letter |
| char | `is_punctuation` | `c.is_punctuation() -> bool` | punctuation |
| char | `is_control` | `c.is_control() -> bool` | control character |
| char | `is_ascii` | `c.is_ascii() -> bool` | code point below 128 |
| char | `to_uppercase` | `c.to_uppercase() -> char` | upper-case char |
| char | `to_lowercase` | `c.to_lowercase() -> char` | lower-case char |
| char | `to_digit` | `c.to_digit(radix: int = 10) -> int?` | digit value: '7'.to_digit() is Some(7) |
| char | `to_string` | `c.to_string() -> string` | one-character string |
| int | `abs` | `x.abs() -> int` | absolute value, e.g. (-3).abs() is 3 |
| int | `pow` | `x.pow(n: int) -> int` | integer power |
| int | `min` | `a.min(b) -> int` | smaller of two, e.g. 3.min(5) is 3 |
| int | `max` | `a.max(b) -> int` | larger of two |
| int | `clamp` | `x.clamp(lo, hi) -> int` | limit to a range |
| int | `signum` | `x.signum() -> int` | -1, 0 or 1 |
| int | `cmp` | `a.cmp(b) -> Ordering` | compare (for sort_by) |
| int | `checked_add` | `x.checked_add(y) -> int?` | None on overflow (also checked_sub, checked_mul, checked_div) |
| int | `wrapping_add` | `x.wrapping_add(y) -> int` | wraps on overflow (also wrapping_sub, wrapping_mul) |
| int | `to_string` | `x.to_string() -> string` | decimal text (in a string: "${x}") |
| int | `to_be_bytes` | `x.to_be_bytes() -> Vec<u8>` | big-endian bytes |
| int | `to_le_bytes` | `x.to_le_bytes() -> Vec<u8>` | little-endian bytes |
| int | `is_digit` | `b.is_digit() -> bool` | byte (u8) is '0'..'9'; the char predicates apply to bytes with the same names |
| int | `is_alphabetic` | `b.is_alphabetic() -> bool` | byte (u8) is an ASCII letter |
| int | `is_whitespace` | `b.is_whitespace() -> bool` | byte (u8) is ASCII whitespace |
| int | `to_uppercase` | `b.to_uppercase() -> u8` | ASCII upper-case byte |
| int | `to_lowercase` | `b.to_lowercase() -> u8` | ASCII lower-case byte |
| float | `abs` | `x.abs() -> float` | absolute value |
| float | `sqrt` | `x.sqrt() -> float` | square root |
| float | `pow` | `x.pow(y) -> float` | power; y may be int or float |
| float | `floor` | `x.floor() -> float` | round down |
| float | `ceil` | `x.ceil() -> float` | round up |
| float | `round` | `x.round() -> float` | round to nearest, ties away from zero |
| float | `trunc` | `x.trunc() -> float` | round toward zero |
| float | `min` | `a.min(b) -> float` | smaller of two |
| float | `max` | `a.max(b) -> float` | larger of two |
| float | `clamp` | `x.clamp(lo, hi) -> float` | limit to a range |
| float | `signum` | `x.signum() -> float` | -1.0, 0.0 or 1.0 |
| float | `cmp` | `a.cmp(b) -> Ordering` | total order, NaN last (for sort_by) |
| float | `sin` | `x.sin() -> float` | sine (also cos, tan, asin, acos, atan, sinh, cosh, tanh) |
| float | `atan2` | `y.atan2(x) -> float` | angle of the point (x, y) |
| float | `exp` | `x.exp() -> float` | e to the x |
| float | `ln` | `x.ln() -> float` | natural logarithm (also log2, log10) |
| float | `log` | `x.log(base) -> float` | logarithm in a base |
| float | `hypot` | `x.hypot(y) -> float` | sqrt(x*x + y*y) |
| float | `mul_add` | `x.mul_add(a, b) -> float` | x*a + b, one rounding |
| float | `rem_euclid` | `x.rem_euclid(y) -> float` | non-negative remainder |
| float | `copysign` | `x.copysign(y) -> float` | x with the sign of y |
| float | `is_nan` | `x.is_nan() -> bool` | not-a-number test |
| float | `is_finite` | `x.is_finite() -> bool` | neither infinite nor NaN |
| float | `is_infinite` | `x.is_infinite() -> bool` | infinity test |
| float | `to_string` | `x.to_string() -> string` | decimal text (in a string: "${x:.2}") |
| float | `to_bits` | `x.to_bits() -> u64` | IEEE-754 bit pattern |
| Option | `is_some` | `o.is_some() -> bool` | true for Some |
| Option | `is_none` | `o.is_none() -> bool` | true for None |
| Option | `map` | `o.map(\|x\| expr) -> U?` | transform the Some value |
| Option | `and_then` | `o.and_then(\|x\| option) -> U?` | chain a step that may be absent |
| Option | `filter` | `o.filter(\|x\| cond) -> T?` | keep Some only when cond holds |
| Option | `or` | `a.or(b) -> T?` | a if Some, else the option b (a value default: a ?? d) |
| Option | `ok_or` | `o.ok_or(err) -> Result<T, E>` | Some(x) -> Ok(x), None -> Err(err) |
| Option | `take` | `o.take() -> T?` | move the value out, leaving None |
| Option | `replace` | `o.replace(x) -> T?` | store x, return the old value |
| Option | `zip` | `a.zip(b) -> (T, U)?` | both values when both are Some |
| Option | `flatten` | `o.flatten() -> T?` | collapse Option<T?> |
| Option | `expect` | `o.expect(why: string) -> T` | the value, or panic with why: for invariants, never for input (no lint) |
| Result | `is_ok` | `r.is_ok() -> bool` | true for Ok |
| Result | `is_err` | `r.is_err() -> bool` | true for Err |
| Result | `ok` | `r.ok() -> T?` | the Ok value, dropping the error |
| Result | `err` | `r.err() -> E?` | the Err value |
| Result | `map` | `r.map(\|x\| expr) -> Result<U, E>` | transform the Ok value |
| Result | `map_err` | `r.map_err(\|e\| expr) -> Result<T, F>` | transform the error |
| Result | `and_then` | `r.and_then(\|x\| result) -> Result<U, E>` | chain fallible steps |
| Result | `expect` | `r.expect(why: string) -> T` | the Ok value, or panic with why and the error (no lint) |
| HashMap | `new` | `HashMap::new() -> HashMap<K, V>` | an empty map (a filled one: {k: v}) |
| HashMap | `insert` | `m.insert(k, v) -> V?` | add or replace an entry; returns the old value |
| HashMap | `get` | `m.get(k) -> V?` | lookup; m[k] panics when k is missing |
| HashMap | `get_mut` | `m.get_mut(k) -> V?` | lookup for changing the value in place |
| HashMap | `contains` | `m.contains(k) -> bool` | key test (same name as list, set and string) |
| HashMap | `remove` | `m.remove(k) -> V?` | delete an entry, returning its value |
| HashMap | `keys` | `m.keys() -> Iter<K>` | the keys (order unspecified: m.keys().sorted() is a sorted list) |
| HashMap | `values` | `m.values() -> Iter<V>` | the values (order unspecified) |
| HashMap | `entry` | `m.entry(k).or_insert(v)` | the value for k, inserting v first when missing: `*m.entry(w).or_insert(0) += 1` |
| HashMap | `retain` | `m.retain(\|k, v\| keep)` | keep entries where the closure is true |
| HashMap | `extend` | `m.extend(other)` | add every entry of another map or pair list |
| HashMap | `clear` | `m.clear()` | remove every entry |
| Entry | `or_insert` | `m.entry(k).or_insert(v) -> V` | the one get-or-insert form |
| BTreeMap | `first` | `m.first() -> (K, V)?` | the entry with the smallest key |
| BTreeMap | `last` | `m.last() -> (K, V)?` | the entry with the largest key |
| BTreeMap | `range` | `m.range(a..b) -> Vec<(K, V)>` | the entries with keys in a range, in order |
| HashSet | `new` | `HashSet::new() -> HashSet<T>` | an empty set (from a sequence: let s: HashSet<T> = xs.collect()) |
| HashSet | `insert` | `s.insert(x) -> bool` | add an element; false when it was present |
| HashSet | `remove` | `s.remove(x) -> bool` | delete an element |
| HashSet | `union` | `a.union(b) -> Iter<T>` | elements in either set (a set: .collect() into a HashSet) |
| HashSet | `intersection` | `a.intersection(b) -> Iter<T>` | elements in both sets |
| HashSet | `difference` | `a.difference(b) -> Iter<T>` | elements in a but not b |
| HashSet | `symmetric_difference` | `a.symmetric_difference(b) -> Iter<T>` | elements in exactly one set |
| HashSet | `is_subset` | `a.is_subset(b) -> bool` | every element of a is in b |
| HashSet | `is_superset` | `a.is_superset(b) -> bool` | every element of b is in a |
| HashSet | `is_disjoint` | `a.is_disjoint(b) -> bool` | no shared elements |
| HashSet | `retain` | `s.retain(\|x\| keep)` | keep elements where the closure is true |
| HashSet | `extend` | `s.extend(xs)` | add many elements |
| HashSet | `clear` | `s.clear()` | remove every element |
| BTreeSet | `first` | `s.first() -> T?` | the smallest element |
| BTreeSet | `last` | `s.last() -> T?` | the largest element |

## Removal list with inventory

Counts of each removed form at `b71288999` (repository) and the ecosystem checkout (`eco`, `eco_docs`); "code total"
excludes the two docs columns. Lexical counts: matches inside strings and comments count, and Rust test files that embed
Windjammer in string literals are not scanned (their fixtures are migrated with the `.wj` corpus by stream S8).
`-` means not lexically countable (type-directed; counted by the checker in S2). Rows with action `pending` wait for
decision B and are not removed.

| id | action | fix | std | tests | examples | golden | bench | scripts | docs | eco | eco_docs | code total | form | replacement |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| R001 | error | auto | 23 | 136 | 16 | 4 | 0 | 1 | 125 | 0 | 2 | 180 | `vec![..]` | [..] / [x; n] / [] |
| R002 | error | auto | 59 | 34 | 19 | 0 | 12 | 2 | 184 | 66 | 0 | 192 | `name!(..) macro call` | println(..), "${x}", debug(x), stderr.println(..) |
| R003 | error | auto | 0 | 532 | 0 | 0 | 19 | 1 | 8 | 15 | 0 | 567 | `"{}" format string in an intrinsic` | "${x}" interpolation |
| R004 | error | auto | 25 | 14 | 4 | 0 | 0 | 1 | 429 | 1 | 1 | 45 | `String / str / &str type` | string |
| R005 | error | auto | 0 | 9 | 2 | 0 | 3 | 0 | 41 | 0 | 0 | 14 | `String::new() / String::from(x)` | "" / x |
| R006 | error | auto | 0 | 54 | 0 | 4 | 0 | 1 | 46 | 1 | 0 | 60 | `"lit".to_string()` | "lit" |
| R007 | error | auto | 131 | 329 | 24 | 14 | 5 | 21 | 292 | 217 | 1 | 741 | `Option<T> in a type` | T? (Option<T?> only for a nested option) |
| R008 | error | auto | 4 | 696 | 6 | 116 | 41 | 184 | 45 | 846 | 5 | 1893 | `Vec::new()` | [] (annotate: let xs: Vec<int> = []) |
| R010 | error | auto | 1 | 68 | 7 | 3 | 8 | 1 | 117 | 0 | 0 | 88 | `.iter() / .iter_mut() / .into_iter()` | nothing: `for x in xs`, `xs.map(..)` |
| R012 | error | auto | 0 | 11 | 23 | 7 | 2 | 0 | 18 | 0 | 0 | 43 | `.cloned() / .copied()` | nothing (elements are values) |
| R015 | error | manual | 0 | 1 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 1 | `.for_each(f)` | for x in xs { .. } |
| R016 | delete | manual | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | `.by_ref() / .fuse()` | nothing: bind the Iter with let mut and keep calling next() |
| R017 | delete | auto | 0 | 4 | 8 | 0 | 0 | 0 | 10 | 0 | 0 | 12 | `.sum::<T>() / .product::<T>()` | .sum() / .product() (the element type decides) |
| R018 | delete | typed | 0 | 1 | 0 | 0 | 7 | 0 | 3 | 0 | 0 | 8 | `.nth(i) on a list` | .get(i) (an Iter keeps nth) |
| R019 | delete | auto | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | `.frequencies()` | .tally() |
| R020 | delete | auto | 0 | 2 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 2 | `.each_slice(n)` | .chunks(n) |
| R021 | delete | auto | 0 | 2 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 2 | `.partition_by(p)` | .partition(p) |
| R022 | delete | manual | 0 | 8 | 0 | 0 | 0 | 0 | 4 | 0 | 0 | 8 | `.zip_with(ys, f)` | .zip(ys).map(\|(a, b)\| f(a, b)) |
| R023 | delete | manual | 0 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | `.find_map(f) / .reduce(f) / .inspect(f) / .rposition(p) / .next_back()` | .filter_map(f).next() / .fold(..) / a for loop / .rev().position(p) |
| R024 | delete | auto | 0 | 1 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 1 | `.to_list() (enumerable)` | .collect() |
| R030 | pending | auto | 10 | 87 | 6 | 30 | 83 | 11 | 33 | 94 | 1 | 321 | `.len() as int / .count() as int (any integer cast of a length)` | under B1: .len() (it is int); under B2/B3 only the `as usize` ones are redundant |
| R031 | pending | typed | 0 | 54 | 0 | 80 | 408 | 31 | 28 | 203 | 3 | 776 | `x as usize in an index or a length argument` | under B1 and B2: nothing at an index (xs[i] converts); under B1 also std length arguments |
| R040 | error | typed | 0 | 106 | 17 | 0 | 28 | 0 | 115 | 163 | 15 | 314 | `.unwrap()` | x? (propagate), x ?? default, match / if let, or x.expect("why") |
| R041 | error | auto | 2 | 168 | 30 | 187 | 614 | 118 | 133 | 88 | 2 | 1207 | `.unwrap_or(d)` | x ?? d |
| R042 | error | typed | 0 | 0 | 0 | 0 | 0 | 0 | 3 | 0 | 0 | 0 | `.unwrap_or_else(f)` | x ?? expr (lazy already); for an error-dependent default: match |
| R043 | error | typed | 0 | 1 | 1 | 0 | 0 | 0 | 1 | 0 | 0 | 2 | `.unwrap_or_default()` | x ?? 0 / x ?? "" / x ?? [] |
| R044 | error | auto | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 29 | 2 | 30 | `.unwrap_err() / .expect_err(m)` | r.err().expect("why") or match |
| R045 | delete | auto | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 0 | 2 | `.map_or(d, f) / .map_or_else(g, f)` | x.map(f) ?? d |
| R046 | delete | auto | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | `.is_some_and(f) / .is_ok_and(f)` | x.map(f) ?? false |
| R047 | delete | auto | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | `.ok_or_else(f) / .or_else(f)` | .ok_or(err) / .or(other) |
| R048 | delete | manual | 0 | 0 | 1 | 0 | 0 | 0 | 1 | 0 | 0 | 1 | `Option .and(b) / .xor(b) / .as_mut() / .insert(x) / .get_or_insert_with(f) / .unzip() / .as_ref() / .as_deref()` | match / if let |
| R050 | error | typed | 0 | 29 | 1 | 0 | 5 | 0 | 20 | 0 | 0 | 35 | `s.push_str(t) / s.push(c) on a string` | s += t / s += c |
| R051 | error | typed | 6 | 1 | 0 | 1 | 2 | 0 | 17 | 0 | 2 | 10 | `s.to_string() / .to_owned() / .as_str() / .into_string() on a string` | nothing (one string type) |
| R052 | delete | auto | 5 | 6 | 0 | 1 | 1 | 9 | 5 | 5 | 0 | 27 | `s.as_bytes() / .into_bytes()` | s.bytes() |
| R053 | delete | auto | 0 | 227 | 2 | 0 | 0 | 9 | 2 | 236 | 0 | 474 | `s.substring(a, b) / s.slice(a, b) / s.get(a..b) / s.char_count()` | s[a..b] / s.chars().count() |
| R054 | delete | auto | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | `s.split_ascii_whitespace() / .split_terminator(p) / .rsplitn(n, p)` | .split_whitespace() / .split(p) / .rsplit(p).take(n) |
| R055 | delete | manual | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | `s.insert(i, c) / .insert_str(i, t) / .remove(i) on a string` | s[..i] + t + s[i..] |
| R056 | delete | manual | 0 | 2 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 2 | `s.capacity() / .reserve(n) / .is_char_boundary(i)` | nothing (storage detail) |
| R057 | error | auto | 0 | 7 | 2 | 0 | 11 | 0 | 1 | 0 | 0 | 20 | `s.to_ascii_uppercase() / .to_ascii_lowercase() / .eq_ignore_ascii_case(t)` | .to_uppercase() / .to_lowercase() / .eq_ignore_case(t) |
| R058 | error | auto | 0 | 1320 | 15 | 0 | 54 | 363 | 21 | 1766 | 3 | 3518 | `std::strings free functions (strings.to_upper(s), strings.split(s, d), ...)` | the method: s.to_uppercase(), s.split(d), ... |
| R060 | error | auto | 0 | 0 | 5 | 1 | 0 | 0 | 0 | 0 | 0 | 6 | `c.is_ascii_digit() / c.is_numeric() / c.is_digit(10)` | c.is_digit() |
| R061 | error | auto | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | `c.is_ascii_hexdigit()` | c.is_digit(16) |
| R062 | error | auto | 0 | 0 | 0 | 1 | 13 | 0 | 0 | 0 | 0 | 14 | `c.is_ascii_alphabetic() / is_ascii_alphanumeric / is_ascii_uppercase / is_ascii_lowercase / is_ascii_whitespace / is_ascii_punctuation` | c.is_alphabetic() / ... (ASCII only: c.is_ascii() && c.is_alphabetic()) |
| R063 | delete | auto | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 1 | `c.len_utf8()` | c.to_string().len() |
| R070 | error | auto | 0 | 16 | 18 | 0 | 0 | 0 | 0 | 0 | 0 | 34 | `std::math free functions (math.sqrt(x), math.abs_i64(x), math.max_f64(a, b), ...)` | the method: x.sqrt(), x.abs(), a.max(b); std::math keeps PI, E, TAU, SQRT_2 |
| R071 | delete | auto | 1 | 3 | 1 | 0 | 0 | 0 | 3 | 0 | 0 | 5 | `x.powi(n) / x.powf(y)` | x.pow(n) |
| R072 | error | auto | 0 | 2 | 2 | 0 | 0 | 0 | 1 | 1 | 0 | 5 | `a.partial_cmp(b) / a.total_cmp(b)` | a.cmp(b) (floats: total order) |
| R073 | error | auto | 8 | 23 | 11 | 7 | 2 | 3 | 25 | 7 | 0 | 61 | `free max(a, b) / min(a, b) / abs(x) (never existed; agents write them)` | a.max(b) / a.min(b) / x.abs() |
| R074 | delete | manual | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | `x.is_sign_negative() / .is_sign_positive()` | x < 0.0 (or x.copysign(1.0) < 0.0 for -0.0) |
| R080 | error | auto | 3 | 6 | 0 | 0 | 0 | 0 | 5 | 1 | 0 | 10 | `.to_vec() / .as_slice() / .into_boxed_slice()` | nothing (a list is a value; xs[a..b] is a list) |
| R081 | error | typed | 2 | 0 | 11 | 0 | 0 | 0 | 0 | 0 | 0 | 13 | `Vec .push_back / .pop_back / .push_front / .pop_front / .front() / .back() / .front_mut() / .back_mut()` | push / pop / first() / last(); VecDeque: push, pop, push_front, pop_front |
| R082 | error | auto | 2 | 2 | 1 | 0 | 0 | 1 | 1 | 0 | 0 | 6 | `.extend_from_slice(&ys) / .append(&mut ys)` | xs.extend(ys) |
| R083 | delete | auto | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | `.sort_unstable() / .sort_unstable_by(f) / .sort_unstable_by_key(f)` | .sort() / .sort_by(f) / .sort_by_key(f) |
| R084 | delete | manual | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | `.first_mut() / .last_mut()` | xs[0] / xs[xs.len() - 1], or xs.get_mut(i) |
| R085 | delete | manual | 0 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | `xs.fill(x) / [x].repeat(n) / xs.dedup_by_key(f) / xs.is_sorted() / xs.split_first() / xs.split_last() / xs.capacity()` | [x; n] / xs.unique_by(f) / xs == xs.sorted() / (xs.first(), xs.skip(1)) |
| R086 | error | auto | 4 | 46 | 17 | 1 | 12 | 3 | 39 | 6 | 2 | 89 | `& / &mut on a std method argument or a map index (xs.contains(&x), m[&k])` | xs.contains(x), m[k] |
| R090 | error | auto | 7 | 25 | 3 | 7 | 39 | 0 | 4 | 5 | 0 | 86 | `m.contains_key(k)` | m.contains(k) |
| R091 | delete | auto | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | `m.into_keys() / .into_values() / .values_mut() / .remove_entry(k)` | m.keys() / m.values() / for (k, v) in m { m[k] = .. } / m.remove(k) |
| R092 | error | typed | 0 | 0 | 2 | 0 | 4 | 0 | 6 | 0 | 0 | 6 | `entry(k).or_insert_with(f) / .or_default() / .and_modify(f) / m.get_or_insert(k, v)` | m.entry(k).or_insert(v) |
| R093 | delete | auto | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | `BTreeMap first_key_value / last_key_value / pop_first / pop_last` | m.first() / m.last() / m.remove(k) |
| R094 | delete | typed | - | - | - | - | - | - | - | - | - | 0 | `HashSet .first() / .last() / .take(x)` | BTreeSet first/last; s.remove(x) |
| R099 | delete | manual | 0 | 1 | 0 | 0 | 0 | 0 | 5 | 1 | 1 | 2 | `__opt_chain (lowering of a?.b)` | a?.b |

Totals: 62 removal rows, 10,934 occurrences in code and 1,845 in docs; 1,097 of the code occurrences are the two
`pending` rows. Of the 9,837 that are removed now, 96% are `auto` rows, 388 occurrences need the checker's types and 14
need a person. The largest are mechanical: `strings.` free functions 3,518, `Vec::new()` 1,888, `.unwrap_or(` 1,207,
`Option<` 741, `"{}"` templates 567, `.unwrap()` 314 (typed).

## Integer-width evidence (decision B)

`wj run --interpret scripts/wj/std_inventory.wj --widths` at `b71288999`:

| kind | std | tests | examples | golden | bench | scripts | docs | eco | eco_docs | code total |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| W1 length cast to usize (a no-op today) | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| W2 length cast to another integer (int, i64, i32, u32, ...) | 10 | 87 | 6 | 30 | 83 | 11 | 33 | 94 | 1 | 321 |
| W3 cast to usize inside an index or slice xs[..] | 0 | 22 | 0 | 54 | 382 | 27 | 13 | 189 | 1 | 674 |
| W4 cast to usize in a std length or position argument | 0 | 2 | 0 | 7 | 11 | 1 | 1 | 0 | 0 | 21 |
| W5 every cast to usize | 0 | 54 | 0 | 80 | 408 | 31 | 28 | 203 | 3 | 776 |
| W6 every cast to int or i64 | 11 | 202 | 17 | 50 | 116 | 61 | 76 | 211 | 2 | 668 |
| W7 a let annotated int whose value calls len()/count() | 0 | 7 | 0 | 0 | 49 | 1 | 29 | 2 | 0 | 59 |
| W8 a let annotated usize | 0 | 19 | 0 | 77 | 1 | 2 | 6 | 3 | 0 | 102 |

## `wj fix`: the one-shot migration tool

One engine, driven by the `removed` table, replacing the per-feature passes in `src/cli/fix.rs` and
`src/intrinsics/surface_fix.rs`:

1. **Lexical rewrites (`fix = "auto"`).** The existing token-aware scanner (strings, comments, chars and `${..}`
   understood) applies each row's rewrite: renames (`contains_key` -> `contains`, `to_list` -> `collect`,
   `strings.to_upper(s)` -> `s.to_uppercase()`), deletions (`.iter()`, `.cloned()`), and shape rewrites
   (`x.unwrap_or(d)` -> `x ?? d`, `x.map_or(d, f)` -> `x.map(f) ?? d`, `vec![..]` -> `[..]`, macros and `{}` templates as
   today).
2. **Typed rewrites (`fix = "typed"`).** Run `wj check` first and read receiver and target types from `src/typing`:
   `.unwrap()` becomes `?` when the enclosing function returns a compatible `Option`/`Result`, else
   `.expect("<expr> is present")`; `s.push_str(t)` / string `s.push(c)` become `s += t`; `.to_string()` on a `string` is
   deleted; every WJ0035 fix of decision H is applied. Under decision B1 the `pending` cast rows join this class.
3. **Manual rows (`fix = "manual"`).** Reported as `file:line:col: R0xx <form> -> <replacement>`; never guessed.
4. **Safety.** A file is written only when it parses before and after and `wj check` reports no more errors after; the
   rewrite is idempotent (`fix(fix(x)) == fix(x)`, a property test over the corpus); `--check` lists what would change
   (CI); `--docs` rewrites Markdown fences; S3's differential gate runs every `examples/verified` and `tests/spec` program
   interpreted before and after the rewrite and requires identical output.
5. **ADR-0002 stays**: the pass that adds a missing `mut` runs last, as today.

## Migration plan

Independent streams; owners follow the model routing rule (opus for design reviews, sonnet for implementation, haiku for
mechanical rows behind a gate). Agent-days are for one agent; builds go through the shared slots.

| Stream | What | Files | Gate | Owner | Est. |
|---|---|---|---|---|---|
| S1 source of truth | toml loader (`include_str!`, `toml` crate), `wj agent-index` generates builtins.txt, G1, G2, G4, G6 wired into `cargo test`; `std/enumerable.wj` and `enumerable.rs` checked against `kind = "enumerable"` rows | `src/stdlib_surface/surface.rs` (new), `src/cli/agent_index.rs`, `tests/std_surface_test.rs`, `tests/agent_index_builtins_coverage_test.rs` | G1, G2 (ledger may only shrink), G4, G6 | sonnet | 2-3 |
| S2 WJ0180 | removed-form check from the table: lexical rows in the checker's token pass, typed rows from `src/typing`; catalog entry, retired W-codes, oracle cases | `src/check/removed_forms.rs` (new), `src/error_catalog.rs`, `src/linter/{surface_syntax,rust_leakage,redundant_as_str}.rs` (deleted into it), `src/diagnostics/codes_lints.rs`, `tests/diagnostics_oracle/check/removed/` | G5; diagnostics goldens | opus review, sonnet | 3-4 |
| S3 `wj fix` | the table-driven engine above, plus `wj check --fix` / `wj run --fix` | `src/cli/fix.rs`, `src/intrinsics/surface_fix.rs` -> `src/fix/{lexical,typed,report}.rs` | idempotence property; parses-and-checks-after; interpreted differential on verified examples and spec cases | sonnet | 4-5 |
| S4 iterator used as a list | decision H's diagnostic and fixes from static types; `collect` target rule; Rust lowering emits `collect::<T>()` with the resolved type; std declarations of the lazy rows (`split`, `keys`, set operations) | `src/check/semantic/checker.rs`, `src/diagnostics/codes_errors.rs` (WJ0035), `std/rust_core.wji`, `src/typing/core_surface.rs`, `src/codegen/rust/*method*`, `src/interpreter/builtins/{iter,seq,text}.rs` (run-time check becomes an assertion), `tests/diagnostics_oracle/check/iter_as_list/` | G8; G3 for every `seq` row; drift corpus (walker / resolved / compiled) | sonnet, opus review | 3-4 |
| S5 integer widths | after the owner decides B. Step 1 (any option): delete the casts of the golden set and the examples, build once, count what the current solver rejects. Then for B1: surface types, boundary conversions in the Rust lowering, retire ADR-0028's index sources and the W5 decider; for B2: automatic conversion at index sites | `src/ir/solver.rs`, `src/ir/constraint_gen.rs`, `src/codegen/rust/usize_expression_type_inference.rs`, `docs/spec/inference.md`, `docs/spec/numeric.md` | golden rustc rejects only fall; narrowing-cast ledger only falls; `D-WIDTH-OWNERS` only falls; `scripts/ecosystem_width_gate.wj` green | opus design, sonnet | 1 + (5-8 for B1, 2-3 for B2) |
| S6 std modules | delete `std/strings.wj` and the free functions of `std/math.wj` (constants stay); regenerate `std_functions_table.rs` | `std/strings.wj`, `std/math.wj`, `src/stdlib_surface/std_functions_table.rs`, `src/interpreter/builtins/std_fns.rs` | `tests/std_functions_surface_test.rs`; G6 rows R058/R070 at 0 | haiku | 1 |
| S7 interpreter registry | remove every removed method from the registry; mark internals; registry == surface | `src/interpreter/builtins/*.rs` | G2 with an empty ledger (the ledger file is deleted) | haiku, sonnet review | 2 |
| S8 corpus migration | `wj fix` over std/, tests/ (including Windjammer embedded in `tests/*.rs` fixtures), examples/, bench/, scripts/, docs/ and the book (`--docs`); manual rows by hand; then the ecosystem in its own repository | the corpora | G6 to 0 outside the ADR and oracle fixtures; full `cargo test`; compiled health sweep; ecosystem gates | haiku (mechanical), sonnet (manual rows) | 3 + 3 (ecosystem) |
| S9 docs and agents | regenerate the index; rewrite the agent sheet, cheatsheets, spec chapters (`stdlib`, `collections`, `optionals`, `strings`, `numeric`), book chapters | `docs/agent/*`, `docs/spec/*`, `docs/book/src/*` | `tests/agent_docs_test.rs`; book doctests; `docs_truth` | sonnet | 2 |

Order: S1 first (everything reads the table). S2, S3 and S4 next, in parallel. S5 waits for the owner's decision B
(its step 1 can run any time and informs that decision). S8 runs per area once S3 lands, and the WJ0035 fixes after S4.
S6 and S7 delete code only after S8 has migrated their callers (their G6 rows at 0). S9 last, with the measurement
below. Total about 24-34 agent-days depending on B; with three build slots, about two to three calendar weeks.

## Effect on the agent sheet and the golden set

`docs/agent/WINDJAMMER_FOR_AGENTS.md` changes:

- "`split`, `split_whitespace`, `lines`, `chars` give an iterator, not a list ... `.collect()` first" stays true and
  becomes cheap: "they are lazy; `.collect()` gives a list. Forgetting it is WJ0035, which `--fix` applies for you";
- "never ... `.unwrap()`" becomes "absence: `x?`, `x ?? default`, `match`; `.expect("why")` for invariants";
- "`len()` is `usize` when compiled: `let n: int = v.len() as int`" is deleted under B1, rewritten under B2, kept under B3;
- added: "run `wj check --fix FILE` / `wj run --fix FILE`: removed forms and a missing `.collect()` are fixed in place".

The golden set (`bench/agent-dx/solutions`, the reference programs the offline levers of ADR-0037 score) is migrated by
`wj fix` in S8: 187 `unwrap_or`, 116 `Vec::new()`, 14 `Option<`, 7 `contains_key`, 7 free `max`/`min`/`abs` calls, plus
the casts decision B removes (30 length casts and 61 `as usize` under B1, 54 under B2). `cost.write` (tokens to write the
same tasks, Windjammer against Rust) is re-measured before and after; the friction corpus (`bench/agent-dx/friction/`) is
re-replayed, and every event whose class no longer costs a turn (missing collect with `--fix`, W0002, E0061, E0282,
interpreter-only methods, and length casts under B1) is relabelled "designed out". The next paid pilot runs on the haiku
arm only after the offline numbers move.

## Risks

- **Fix placement.** An `Iter` bound once and used in two places needs its `.collect()` on the `let`, not at the first
  use; G8 has a case for each placement, and the fix is checked to type-check before it is offered.
- **Interpreter divergence today.** The interpreter accepts `.len()` on a `split` result that the compiled build rejects;
  S4 moves the decision to the shared checker, and the drift corpus gates that both engines agree.
- **Ecosystem churn.** 3,518 `strings.` calls and most `Vec::new()` sites are in the ecosystem; all are `auto` rows, run
  by `wj fix` in its own repository with its own gates.
- **Behaviour changes.** Unicode-aware predicates (R057, R060, R062: 40 sites), eager `ok_or` (R047: 0 sites), string
  `substring` was character-indexed in the interpreter (R053: 474 sites, rewritten to a character-safe form when the
  receiver may be non-ASCII). Each is listed in the row's `note` and printed by `wj fix`.
- **Decision B.** Until it is taken, lengths keep today's behaviour and the agent sheet keeps its cast rule; S5's step 1
  is the cheapest way to turn the open question into a number.
- **First drafts.** Agents will still write `unwrap`, `iter` and `Vec::new()`; each is one WJ0180 with a
  machine-applicable edit, applied by `--fix` without a turn.

## Decision needed from: project owner

1. **Decision B:** B1 (lengths and positions are `int`, the compiler converts), B2 (lengths stay `usize`, index sites
   convert), or B3 (status quo). Recommendation B1, medium-high confidence; S5 step 1 can measure first.
2. **Decision I:** keep `Vec` / `HashMap` / `HashSet` rather than renaming to `List` / `Map` / `Set`.
3. Accept the removal list and the stream plan; S1 can start on acceptance.
