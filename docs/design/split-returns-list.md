# Should `split` return a list?

Status: open question for the owner. Nothing here is implemented.

## The class of failure today

`"a b c".split(" ")` (and `split_whitespace()`, `lines()`, `chars()`) return an iterator, as the std declaration
(`std/rust_core.wji`, `String::split -> Iterator<&str>`) says and as Rust does. Agents write `parts[0]`, `parts.len()`
or `parts.contains(..)` on the result. The interpreter used to run those programs and the compiled build failed with raw
rustc errors (12 of 40 Windjammer pilot-7 trials). `wj check` / `wj run` now reject them with WJ0035 / WJ0015 and a
machine-applicable `.collect()` fix, so the failure is cheap, but the rule still has to be learned.

## Option A: keep the iterator (today)

Pros
- One meaning, the same as Rust: laziness, no allocation for `for part in s.split(" ")`, `.map(..)` chains stay lazy.
- The std declaration is the single source of truth; the checker decides from it (no special cases).
- Costs nothing at runtime for the common loop and `count()` cases.

Cons
- A rule an agent must learn and get wrong once per program (about 0.8 failing rounds per trial in pilot 7).
- `wj fix` can only repair the program after the mistake, never prevent it.

## Option B: `split` returns `Vec<string>`

Pros
- Removes the whole class: `parts[0]`, `parts.len()`, `parts.contains(..)` just work, as in Python/Go/JS.
- Removes the borrowed-versus-owned element class too (`Vec<string>` of owned pieces; no WJ0029 / `.map(|s| s.to_string())`).
- The interpreter and compiled build agree without a type-directed lowering.

Cons
- Allocates a list and a `String` per piece even for `for part in s.split(" ")` and `.next()` / `.count()` uses; needs an
  optimisation (fuse `for x in s.split(..)` back to the lazy form) to stay competitive on the benchmarks.
- Diverges from Rust: `split(..).rev()`, `.take(n)`, `.skip(n)` become list operations; code ported from Rust changes meaning
  slightly (eager vs lazy, no early exit on huge inputs, infinite iterators impossible).
- `lines()` and `chars()` of a large input would copy the whole input into a list before the loop starts.
- Every `split*` variant (`splitn`, `rsplit`, `split_terminator`, `char_indices`..) would need a decision, or the
  language ends up with two families.

## Middle path (cheapest to try first)

Keep iterators, and let indexing / `len` / `contains` on a std iterator be a *lowering*, not an error: collect at the use
(`parts[1]` -> `parts.clone().collect::<Vec<_>>()[1]`). Costs an allocation per use and hides the model; the diagnostic
plus fix we ship now is the honest version of this.

## Recommendation

Decide by measuring: pilot-7 style trials with Option B as an interpreter-and-compiler flag on the
`wj run --verify` loop, against the allocation cost on `bench/` string-processing programs. Keep Option A until the
numbers say the failure class is worth the eager-allocation cost.
