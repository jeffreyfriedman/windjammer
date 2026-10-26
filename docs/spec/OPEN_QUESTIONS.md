# Open questions: where the implementation and the decisions disagree

Each entry is a place where the implementation contradicts an accepted ADR or the old grammar, or where the behaviour is ambiguous. The spec does **not** silently pick a semantics: the rule text says what is recommended, and the executable example for it is marked `// xfail: Q-NNN` (it must fail today and the run fails if it starts passing, so this list cannot rot). Repro for every question is the named example under `tests/spec/` (or `docs/spec/repros/`). "Rec." is the recommendation.

## Q-001 Escape sequences and the old grammar
Old `docs/windjammer.ebnf` lists `\xNN`; the lexer maps any unknown `\c` to `c` (so `"\x41"` is `x41`). Rule SPEC-LEX-010 records the implementation. Rec.: reject unknown escapes with an error and add `\u{...}`; hex escapes only if byte strings arrive. Repro: `tests/spec/lexical/SPEC-LEX-010.wj`.

## Q-002 Block comments
**Resolved (wt/parser-gaps2):** nested `/* */` implemented in the lexer and formatter (SPEC-LEX-015 passes).
Old grammar documents nested `/* */`; the lexer has no block comments (`/*` is `/` `*`, a parse error). Rec.: implement nesting (Rust-like, since users paste Rust). Repro: SPEC-LEX-015 (xfail).

## Q-003 `int(x)` / `float(x)` checked conversions
ADR-0015 section 2 (Accepted) says these replace `as`. The parser rejects `int(a)` (`int` is a type keyword token). `as` is the only conversion. Rec.: parse `int(..)`/`float(..)` as conversion calls; keep `as` until `wj fix` migrates. Repro: SPEC-NUM-011 (xfail).

## Q-004 `as` versus prefix minus
**Resolved (wt/parser-gaps2):** prefix operators bind tighter than `as` (`-1 as u8` is 255; SPEC-EXPR-005 passes).
`as` is a postfix operator here, so `-1 as u8` is `-(1 as u8)` and panics; Rust and C give `(-1) as u8 = 255`. Rec.: make `as` bind looser than prefix unary operators. Repro: SPEC-EXPR-005 (xfail).

## Q-005 `loop` with `break value` as an expression
**Resolved** by the parser-gaps merge (481081ee): `loop` is an expression with `break value`. SPEC-EXPR-016 is now a normal example.

## Q-006 `to_string(self)` provides Display (ADR-0015 section 3, Accepted)
**Resolved in the interpreter** (interp-gaps2): interpolation uses a user `to_string(self)` as Display; SPEC-STR-009 is a normal example. The Rust backend still needs the generated `impl Display` bridge.

## Q-007 Explicit `let mut` (ADR-0002, Accepted) is not enforced
The Rust backend still infers `mut` for locals (`auto_needs_mut`), and the interpreter accepts assignment to an immutable `let`. ADR-0002 migration (diagnostic + `wj fix`) has not landed. Rec.: front-end diagnostic "`x` is assigned but not declared `mut`" (also the first concrete step of ADR-0005 option C). Repro: SPEC-STMT-002 (xfail).

## Q-008 Range patterns
**Resolved** by the parser-gaps merge (481081ee): range patterns parse and run (interpreter, Rust, portable backends; corpus programs 152-159). SPEC-PAT-008 is now a normal example.

## Q-009 Privacy is not enforced by the interpreter
`m::secret()` from outside `mod m` runs. The Rust backend relies on rustc. Rec.: a resolver check (depends on the DefId resolver work). Repro: SPEC-MOD-004 (xfail).

## Q-010 Generic type aliases
**Resolved (wt/parser-gaps2):** `type Name<T> = ..` accepted; Rust backend emits it (SPEC-ITEM-011 passes; portable backends do not expand aliases, docs/known-issues/portable-type-aliases.md).
`type Pair<T> = (T, T)` is a parse error. Rec.: accept generic parameters on `type`. Repro: SPEC-ITEM-011 (xfail).

## Q-011 Struct field defaults
Old grammar has `f: T = expr` in struct fields; the struct parser rejects it. Rec.: either implement (useful for configs and `Default`) or remove it from docs for good. Repro: SPEC-ITEM-008 (xfail).

## Q-012 Lexer panics on stray characters
**Resolved** by the lexer merge (e0ce8351, `Token::Invalid`): `~0` is now a located parse error (`Lexical error: unexpected character '~'`) instead of a panic. Remaining design question: whether to add a bitwise-not operator (`~` or `!` on integers); not needed for any accepted ADR. Repro kept: `docs/spec/repros/Q-012_tilde_panics_lexer.wj`.

## Q-013 Concurrency surface (ADR-0004, Accepted) is unimplemented
**Resolved in the interpreter** (interp-gaps2): `spawn f(x)`/`spawn { }`, `async f(x)`, `scope { }`, `Task.wait()`/`.await` and `std::sync::channel()` with `send`/`recv` methods run on the deterministic cooperative scheduler (`--sched-seed N` explores interleavings); SPEC-CONC-001..004 are normal examples. The parser lowers the prefixes to `SpawnCall`/`AsyncCall` and `scope` to a runtime call; the Rust/Go/JS backends do not implement them yet (ADR-0004 staging).

## Q-014 Diagnostics contract (ADR-0005, Proposed)
Type/borrow errors come from translated rustc output; `wj check` does not check `.wj` without cargo. Rules that say "compile-time error" (SPEC-STMT-002, SPEC-TYP-003, SPEC-MOD-004) therefore cannot be tested through the front end today. Rec.: ADR-0005 option C.

## Q-015 Interpreter: a `mut` parameter writes through to the caller
**Resolved** (interp-gaps2): a `mut` parameter is an owned copy in the interpreter; only inferred `&mut` parameters write through. SPEC-OWN-008 is a normal example.

## Q-016 Interpreter: `@derive(Default)` unsupported
**Resolved** (interp-gaps2): `Type::default()` for `@derive(Default)` structs runs in the interpreter. SPEC-ATTR-002 is a normal example.

## Q-017 Mixed-width arithmetic is not rejected by the interpreter
`let c = a_i32 + b_i64` runs (the interpreter widens). ADR-0015 section 1 says mixed widths require explicit conversion. The check belongs in the analyzer (Q-014). Repro: SPEC-TYP-003 (xfail).

## Q-018 Float formatting beyond ~1e16 differs between backends
Rust prints `1000000000000000000000`; Go and JavaScript print exponents (`1e+21`). Also `{:.N}` ties: Rust/Python round half to even on the exact binary value, JavaScript `toFixed` differs in places. Corpus programs `55_float_format`, `101` pin the safe range. Rec.: declare the guaranteed range in SPEC-NUM-013 (|x| < 1e16) or make the portable layer implement Rust's algorithm. No xfail (the interpreter agrees with Rust).

## Q-019 Line-initial `[`
**Resolved (wt/parser-gaps2):** line-initial `[` starts a new statement (SPEC-GRAM-004 passes).
docs/design/statement-boundaries.md (normative table, merged with parser-gaps) says a line that begins with anything other than `.`, `*` (non-assignment) or a binary operator starts a new statement, so `f()` newline `[1, 2].len()` is two statements. The parser still reads the `[` as an index into the previous line (`Expected RBracket, got Comma`). Rec.: implement the documented rule (add `[` to the new-statement set, as `(` already is). Repro: SPEC-GRAM-004 (xfail).

## Q-020 Loop labels
**Resolved** by the parser-gaps merge (481081ee): `'label:` on `for`/`while`/`loop`, `break 'label [value]`, `continue 'label`. SPEC-GRAM-008 now specifies them.

## Q-021 Operator precedence differs from the old grammar and from Rust
The old EBNF gave Rust's table. The parser has: shifts tighter than `+`, bitwise `& ^ |` looser than comparisons, relational tighter than equality. Documented as SPEC-EXPR-001..003 (no xfail, they match the parser). Rec.: owner decision; if the bitwise-vs-comparison order should match Rust, `a & b == c` stops being a footgun (it is a type error today for integers).

## Q-022 Interpreter is the only executor for these examples
Examples run in-process via the interpreter. The Rust/Go/JS backends are not run on them (ADR-0007 tiers are Proposed). Rec.: add a `tests/spec` corpus root to the multi-backend harness (`tests/common/conformance`), reusing the same `// expect:` lines. This is the path to making SPEC the conformance gate.

## Q-023 Interpreter recursion cap
**Resolved** (interp-gaps2): on the interpreter's big-stack thread (CLI, `wj test`, REPL, and any interpreter created on that thread) recursion is bounded by stack bytes (about 120k frames in a debug build, far more in release), not a frame count; other threads keep a 10,000-frame guard. SPEC-EXEC-006 is a normal example.

## Q-024 Trait bounds cannot take type arguments
**Resolved (wt/parser-gaps2):** bounds take type arguments in `<..>` and `where` (SPEC-TRT-008 passes on the interpreter; Rust ownership inference still borrows the bounded parameter, docs/known-issues/rust-into-bound-param-borrowed.md).
`fn f<T: Into<string>>(x: T)`, `where T: Clone + Into<string>`, `trait A: B<int>` and `trait V<T: Bound>` are parse errors; only plain trait names are bounds (`struct S<T: Clone>` and `fn f<T: A + B>` work). This blocks idiomatic `Into`, `From`, `Iterator`, `Add<Output=..>` bounds. Rec.: parse `Name<types>` in bounds everywhere (the parser already parses type arguments in types). Repro: SPEC-TRT-008 (xfail); grammar.ebnf `bound`.

## Q-025 Generic methods in traits
**Resolved (wt/parser-gaps2):** generic methods in traits parse and are emitted (SPEC-TRT-007 passes).
`trait T { fn m<U>(self, x: U) }` fails at the `<` (impls may have generic methods). Rec.: share the function-signature parser. Repro: SPEC-TRT-007 (xfail).

## Q-026 `return` followed by a newline swallows the next statement (semantic bug)
**Resolved (wt/parser-gaps2):** `return` takes a value only on the same line; `break`/`continue` labels only on the same line (SPEC-STMT-011 passes).
`return` NEWLINE `println("x")` parses as `return println("x")`: the call runs, then the function returns. Silent wrong behaviour for any "early return" without value followed by more code in the same block. Rec.: a bare `return` ends at a newline when the next line cannot continue an expression (or always require the operand on the same line). Repro: SPEC-STMT-011 (xfail). Same family: a bare `break` followed by a line that starts with a label (`'inner: while ...`) takes the label as its own and fails to parse (found by the spec grammar's random corpus). The grammar models both by ending bare `return`/`break`/`continue` with `;`.

## Q-027 Rust backend: unannotated integer literals are `i32`, not `int` (i64)
`let big = 9223372036854775807` becomes `9223372036854775807_i32` (does not compile) and `let a = 2147483647; a + 1` is `i32` arithmetic. ADR-0015 section 1 (Accepted) and SPEC-NUM-001/TYP-008 say `int` = i64. This is the root of the `rust-int-literal-width-inference` known issue (27 `SUPPORT.toml` entries). The interpreter and JS agree with the spec. Rec.: make the numeric solver default unconstrained literals to i64 (ADR-0015 WP8). Evidence: `scripts/spec_backends.sh` rows NUM-001..004, 006; BACKEND_STATUS.md.

## Q-028 Rust backend: unannotated float literals are `f32`
`let x = 0.1 + 0.2` generates `0.30000000000000004_f32`, so `x == 0.3` is `true`; SPEC-NUM-007 and ADR-0015 say `float` = f64 (`false`). Also affects printed precision. Rec.: default to f64. Evidence: SPEC-NUM-007 FAIL on Rust.

## Q-029 `defer` is not implemented on the Rust backend
The generator emits `// TODO: defer not yet implemented` (once per deferred statement, repeated) and runs nothing. Parsed, interpreted, and documented by SPEC-STMT-007, but silently dropped by the Rust target. Rec.: implement with a drop guard, or make the Rust backend reject `defer` with an explicit error until then (silent drop is the worst option). Evidence: SPEC-STMT-007 FAIL on Rust; JS BUILD.

## Q-030 User functions that share an intrinsic's name are ignored on Rust
`fn println(s: string)` is shadowed by the intrinsic on the Rust target although docs/design/surface-syntax.md says a user function shadows it on every backend. Evidence: SPEC-INTR-009 FAIL on Rust (passes interpreter, JS).

## Q-031 Rust backend: function in a nested `pub mod` is not found
`mod geo { pub mod inner { pub fn one() } }` called as `geo::inner::one()` fails with E0425 (`cannot find function one in module geo::inner`). Evidence: SPEC-MOD-001 BUILD on Rust.

## Q-032 Effects and capabilities: stage E1 done, E2-E7 open (ADR-0021)
ADR-0021 (accepted 2026-10-12) specifies effect inference, grants, capability values, `durable fn` and record/replay in stages E1-E7. **E1 is implemented** (sink table, effect pass, `wj explain-effects`, `wj check` WJ0040-WJ0044, interpreter gate with exit 126, local profile in warn mode): SPEC-EFF-001..011, 013..016, 018..020 pass (policy rules included); E2 adds `std::cap` values (012), per-package grants, the lock file and `console` gating. Still `xfail Q-032`: `durable fn` (021..026; E4), record/replay (029; E3). Rec.: continue with E2 (per-package grants, `@uses` on traits, `std::cap`) against these examples. Repro: the `xfail: Q-032` examples under `tests/spec/effects/`.
