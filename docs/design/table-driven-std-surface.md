---
status: current
last_verified: 8a9941cacf68
---
# Table-driven std surface: one declaration per std function, backends as interpreters

> **Stages 1, 2, 3, 4 and 5 of 9 are implemented** (`src/stdlib_surface/lowering.rs`, `src/codegen/portable/template.rs`, the `Float`
> `Integer`, `Char`, `String`, `Option`, `Result`, `HashMap` and `HashSet` rows of `std/rust_core.wji`, gates G1-G6 and G8 in `tests/std_lowering_table_test.rs`, G2 in
> `tests/portable_emit_golden_test.rs`, G3 in `tests/std_row_examples_test.rs`). The other types, the portability diagnostic and the
> module-function rows are stages 5-8 below. Counts are measured at `8a9941cac` (`wt-int-b10`) with the real scanner
> (`tests/common/heuristic_scan.rs`, run standalone over each file).
>
> What stage 1 settled that the text below leaves open: the signature stays in `CoreSurface` (the `support`-layer
> table cannot import the parser's `Type`), so a `Row` holds the parameter names, whether the result is `T`, whether
> the receiver is `&mut`, the rules and the example, and the typer reads the result type from `method_signature`;
> `{x:f64}` converts an operand to the compute type the trait's `@go(widen = "f64")` names (a result declared `T` is
> converted back), so the Go emitter spells no `f32` rule; `TypeScript` uses the JavaScript rules; the surface is parsed once per process
> (`ir::stdlib_tables::shared_core_surface`, kept with the stdlib registry's derived tables, so no new static) and
> shared by the solver, the Rust backend, the check pass and the portable backends, which makes the decorators' extra
> parse cost (+370 us) a net saving; the lowered module carries its table (`PModule::lowering`); the gates read `std/rust_core.wji` from disk so a
> planted mistake fails without rebuilding the compiler.
>
> What stage 2 settled: the facts of section 2.3 are real (`{T.bits}`, `{T.signed}`, `{T.wide}`, `{T.min}`, `{T.max}`,
> `{T.zero}`, `{T.one}`, `{T.chk}`, `{T.go}`, `{R.go}`), answered by an `Operands` per emitter over its own numeric helpers
> (JavaScript `num::*`, Go `types::int_name`); a fact a target does not have is a refusal, not a guess. Two facts were
> added, `{T.wrap_open}` / `{T.wrap_close}`, because a template is linear and the JavaScript wrap of an expression into
> `T`'s range brackets it (`BigInt.asIntN(64, ..)`, `((..) | 0)`, `(((..) << 24) >> 24)`). The `wrapping_mul` guards are
> `t_i32` and `t_u32` (the `Math.imul` forms) and `t_wide` (a 64-bit receiver, a BigInt) replaces `if wide` in the
> JavaScript text; they read the receiver's own type, which both emitters answer through `template::type_guard`.
> `lowered()` is the one row interpreter for `Float` and `Integer`. A method the emitters never handled
> (`swap_bytes`, `count_zeros`, `checked_neg`, `checked_pow`, `checked_shl`, `checked_shr`, `wrapping_pow`,
> `saturating_pow`, `reverse_bits`, and `isqrt`, whose runtime helper would change every emitted program: the shims are part of the emitted output that G2 hashes) is declared with `@js(unsupported = ..)` / `@go(unsupported = ..)`; G3 pins that list
> (shrink-only), and builds each unsupported row to see the emitter refuse it with the row's reason. `@example(given = "let x: u8 = 250", ..)` declares the typed local an example needs
> (a literal receiver is folded by Go's compiler and takes its type from inference); `@rustc(on = "u64")` names the type
> an entry is checked against rustc on when it exists only on some integer types (`is_power_of_two`, `abs_diff`).
>
> What stage 3 settled: `trait Char` is new (checked against rustc on `char`; `@rustc(tail = ".collect::<String>()")` is the adapter
> the gate adds to Rust's iterator-valued `to_uppercase` / `to_lowercase`, which the surface types as the string they spell) and
> `trait String` gained `char_indices`, `repeat`, `replace`. `parse<F>` takes its target from the expected result (the method
> generic is bound against the expected `Result<T, string>`) and each emitter selects its parser with the `ok_int`, `ok_f32`,
> `ok_f64`, `ok_bool` guards, which `type_guard` answers from the call's result type; `{O.go}`, `{O.lo}`, `{O.hi}`, `{O.wide}` are
> facts about its `Ok` type (`lo` / `hi` are the bounds as BigInt literals, what the JavaScript parser compares in). The modifiers
> `str` (a `char` operand where Go wants a string, `string(rune(c))`) and `num` (an integer JavaScript wants as a Number)
> and the rule `assign` (JavaScript's place update, `(s = s + c, undefined)`) are what `push`, `clear`, `repeat` and the
> string-argument methods needed. The ADR-0039 spellings the emitters used to accept (`to_owned`, `as_str`, `into_string`,
> `as_bytes`, `substring`, `to_ascii_*`, `is_ascii_*`, `is_numeric`, `len_utf8`) have no row: no program of the corpus reaches
> them (the emit hashes stay identical), `clone` / `to_owned` never reached an emitter (they lower to a copy), and a portable
> backend now refuses the rest. `push_str` and `capacity` stay declared (the `let mut` check reads their receiver mode) and are
> `unsupported` on both portable targets; `clone` and `slice` are `@internal` rows (a copy, and the parser's spelling of `x[a..b]`).
> An associated function (`String::new`) is not a method row: statics are stage 5. Rows are looked up by type then name, so a
> lookup allocates nothing.
>
> What stage 4 settled: `trait Option<T>` and `trait Result<T, E>` carry the rows. A closure parameter is declared `fn(T) -> U`
> and a method generic takes its type from the argument: `StdSurface::declared_ret(recv, name, args, want)` binds the trait's
> generics from the receiver (`Option<T>`'s `T`; `Result<T, E>`'s `T` and `E`), then every method generic from the arguments'
> types (`map<U>(f: fn(T) -> U)` binds `U` to the closure's result, `ok_or<E>(err: E)` to the argument), then from the expected
> result (`parse<F>`); `declared_arg` gives a closure its declared parameter types (`fn(T) -> ?`, a result it cannot know yet is
> `Unknown`). The debug text of a `Result`'s error, which `unwrap` / `expect` print in JavaScript (`$unwrapRes(r, $dbgStr)`), is
> the fact `{E.dbg}` (`Fact::Dbg`, asked of `TypeRef::Err`, the receiver's error type): the emitter answers it with its own
> `dbg_fn`, only when a template names it, and Go has no such fact. `unwrap`, `unwrap_or` and `unwrap_or_else` are `@internal`
> rows with rules: `x?` lowers to a match and then the `unwrap` of the checked local (`portable/flatten.rs`), and the parser lowers
> `x ?? d` to `unwrap_or` (a literal or variable default) or `unwrap_or_else` (any other). The spellings ADR-0039 removed and
> no desugaring produces have no portable lowering: `map_or`, `is_some_and` (R045, R046) and `expect_err` (R044) have no
> declaration (a portable backend refuses them as an unknown method; the corpus never reached them), `unwrap_or_default`,
> `cloned`, `copied` and `take` stay declared for the Rust side and are `unsupported` on both portable targets. Their JavaScript and
> Go helpers (`$optMapOr`, `wjOptMapOr`, `wjOptIsSomeAnd`) are dead and listed in the dead-helper lists with the char ones.
>
> What stage 5 settled: `CoreType::Map` and `CoreType::Set` (traits `HashMap`, `HashSet`; `Ty::Map` and `Ty::Set` are also the ordered
> `BTreeMap` and `BTreeSet`, so those traits carry only their associated functions). An associated function or constant is a row
> of its trait with no receiver (`LoweringTable::statics`, a row on any trait that has rules: `Vec` is not migrated yet but its
> `new`, `with_capacity` and `from` are rows). `@owner(names = "i8, ..")` on a trait names the type spellings its functions are called
> on (`Integer` for the ten integer types, `F32`, `F64`, `Char`, `String` for `string`); a constant is a function without parameters
> marked `@constant` (the rustc gate writes `i64::MAX`, not a call). `StdSurface::declared_static_ret` types a call from the
> declaration: the trait's generics from the owner when it is a primitive (`Integer<T>` of `i64`), else from the expected type
> (`HashMap<K, V>` against `Map<string, i32>`), unknown when nothing says; `declared_static_arg` wants a parameter's declared type
> (`String::from` a string, `from_str_radix` a `u32`: its literal radix is a `u32` now, not the `i64` default). New vocabulary (G6
> pins it): `Guard::KeyPrim` with the trait-level `@js(requires = "key_prim", reason = "..{K}..")` (a JavaScript `Map` compares only
> integer, string, char and bool keys by value; replaces `key_ok`), `Hook::Cast` (`i64::from(x)`, `f64::from(x)`: `num::cast` from
> the argument's type to the call's), and `{R.go}` of a non-integer result is the Go spelling of the type (`map[string]int64{}`).
> The spellings ADR-0039 removed (R010 `iter`, `iter_mut`, `into_iter`; R091 `into_keys`, `into_values`, `values_mut`; R030 `capacity`)
> have no declaration (`HashMap::capacity` and `HashSet::capacity` declarations are deleted, so `REMOVED_SPELLED_WITHOUT_INTERNAL` shrank
> by two); `contains_key` stays (the corpus writes it and `m.entry(k).or_insert(v)` lowers to it). Go's `wjEntries` is dead and listed.

## 1. Problem, in numbers

The heuristic surface is 1,740 (red; amber <= 1,200, green <= 500). The portable backends (Go, JS/TS) decide every
std call by matching the method name in Rust code. Per file and per function (match-arm names, plus name comparisons):

| file | total | per function |
|---|---|---|
| `src/codegen/portable/builtins.rs` (typing) | 260 | vec_ret 43, int_ret 34, float_ret 34, str_ret 33, iter_ret 32, char_ret 20, map_ret 15, option_ret 13 (+1 `name ==` in option_arg_want), result_ret 13, static_ret 11, set_ret 7, ret_ty 4 |
| `src/codegen/javascript/emit/builtin.rs` | 246 | vec 38, float 34, iter 32, int 31, str 30, char 20, map 15, static 15, option 11, result 11, set 7, method 1, sum_op 1 |
| `src/codegen/go/emit/builtin.rs` | 238 | vec+iter 68, float 34, int 31, str 30, char 18, map 15, static 12, option 11, result 11, set 7, method 1 |
| `src/codegen/portable/stdlib.rs` (module fns) | 68 | math 33, strings 22, fs 9, time 4 |
| enumerable / ordering, three backends | 93 | portable 9 + 12, JS 18 + 16, Go 22 + 16 |

That is 905 of 1,740 in one family. The same knowledge also lives in uncounted forms the scanner does not see:
tuple patterns in `arg_want` (`("insert", 0) =>`, about 81 literals), `float_const` (13), `stores_args` (12), Go's
`mutating` list (18) and the `matches!(name, "len" | "count" | "capacity")` narrowing in both emitters (6): about
130 more name literals that decide behaviour.

Each fact is written three or four times: the result type of `xs.get(i)` is in `builtins.rs`, the Rust signature in
`std/rust_core.wji`, the JS form in `builtin.rs`, the Go form in `builtin.rs`, and the interpreter's form in
`src/interpreter/builtins/seq.rs`. Adding a method means touching all of them; forgetting one is a silent tier-2 gap.

## 2. Decision

Every std method and std function is declared **once**, in the Windjammer declaration the repo already parses, with its
type signature, its receiver/argument ownership, its effects, and one lowering rule per target. The three backend
emitters and the portable typer become interpreters of that table. No Rust code in `src/codegen/{portable,javascript,go}`
spells a std method name.

### 2.1 Where the table lives (decided: the existing declarations, not a new TOML)

| source | already | becomes |
|---|---|---|
| `std/rust_core.wji` | typed signatures of the core types, parsed by `src/typing/core_surface.rs` with the real parser, every entry checked against rustc (`tests/core_surface_rustc_test.rs`), read by the numeric solver | the **method table**: signature + `&mut self` + owned/borrowed params + per-target decorators |
| `std/<module>.wj` (`fs`, `time`, `json`, ...) | the module function signatures and ADR-0021 effect decorators (`@uses(fs.read)`) | the **function table**: same decorators on each `pub fn` |
| `std/enumerable.wj` | the enumerable signatures, mirrored by `src/stdlib_surface/enumerable.rs` | the enumerable rows (the Rust mirror is generated from it) |
| `docs/std_surface.toml` | ADR-0039's Windjammer-facing table: canonical names, docs, `removed` rows, `lower` to the Rust name | unchanged role; gains a check that every canonical row resolves to exactly one declaration above |

Why not a dedicated TOML: the signatures already exist in typed Windjammer form, are rustc-gated, and are what the
solver reads. A separate lowering TOML would be a second place keyed by name and would drift. `std_surface.toml` stays
the user-facing truth (names, docs, removals); the `.wji`/`.wj` files are the machine truth (types, effects, lowering).
`TraitMethod` and `FunctionDecl` already carry `decorators: Vec<Decorator>` with named arguments, so no grammar change.

### 2.2 Row format

```
pub trait Vec<T> {
    @js(emit = "$vget({self}, {index})")
    @go(emit = "wjGet({self}, {index})")
    fn get(self, index: usize) -> Option<&T>;

    @js(emit = "{self}.push({value})")
    @go(emit = "wjPush({&self}, {value})")
    fn push(&mut self, value: T);

    @js(emit = "{self}.includes({value})", when = "elem_prim")
    @js(emit = "{self}.some((x) => $eq(x, {value}))")
    @go(emit = "wjContains({self}, {value})", when = "elem_prim")
    @go(emit = "wjContainsEq({self}, {value})")
    fn contains(self, value: &T) -> bool;
}

// std/json.wj
@uses(none)
@js(unsupported = "std/runtime/js/wj_json.mjs exists but is not wired to the portable lowering (stage 7)")
@go(unsupported = "std/runtime/go/wj_json.go exists but is not wired to the portable lowering (stage 7)")
pub fn parse(text: string) -> Result<Value, string>
```

Per target (`rust`, `js`, `go`; `ts` inherits `js` unless it has its own decorator), exactly one of:

- `emit = "<template>"`, optionally several with `when = "<guard>"`, first match wins, the last has no guard;
- `assign = "<template>"`: JS place update `({place} = <template>, undefined)` (string `push`, `clear`);
- `hook = "display" | "clone"`: the two type-directed emitters that already exist (`display`, `clone_js` / `cloner`);
- `unsupported = "<reason>"`: the reason is the diagnostic text and the ledger reason.

`rust` defaults to the native method of the same name (the `.wji` entry is a Rust method, rustc-checked); an explicit
`@rust(emit = ...)` covers the differences `std_surface.toml` lists as `lower` (VecDeque `push` -> `push_back`). A module
file may carry one file-level `@js(unsupported = ...)` / `@go(unsupported = ...)` that applies to every function
without its own rule (the Rust-only modules in `std/README.md`: http, net, db, ...), which is explicit, not a default.

What the row decides from declarations instead of names:

| today (name list) | from the declaration |
|---|---|
| `ret_ty` / `arg_want` tables | the signature, instantiated by `CoreSurface::method_signature` (receiver generics, `fn(T) -> U` closure params, turbofish); want-typed results (`collect`, `sum`, `parse`) declare a method generic taken from the expected type: `fn collect<C>(self) -> C` with `@result_from_context` |
| `stores_args` | a parameter declared owned (`value: T`) is stored; `&T` is borrowed |
| Go `mutating` list | `&mut self` (already the ADR-0002 source for WJ0051) -> `{&self}` |
| `len` / `count` / `capacity` narrowing | the declared result is `usize` (ADR-0028 index slot) and the call's type is a narrower int: one rule for every such method |
| Go `arg_str` (char argument where a string is expected) | the parameter's declared type plus the `:str` placeholder modifier |
| JS `key_ok` on map keys | a declared bound on the trait (`HashMap<K: Key, V>`) |
| Go f32 widening in float methods | trait-level `@go(widen = "f64")` on `Float<T>` |
| W0003 (list calls iterator methods directly) | `@adapts(Iterator)` on `Vec<T>` / `VecDeque<T>`: lookup falls through to the `Iterator` trait |

### 2.3 Template language (closed, small)

- `{self}` receiver value; `{&self}` receiver place (Go `&x`, JS assignable place); `{name}` an argument by its declared
  parameter name; a modifier `{name:str}` (char -> string), `{name:f64}` (Go widen), `{name:num}` (JS `Number(..)`).
- type facts of the receiver's `T` or the result `R`: `.bits .signed .wide .min .max .zero .one .chk .go`
  (`{T.max}`, `{R.go}`), all computed by the existing `num` helpers per backend.
- guards: `elem_prim`, `elem_prim_not_bool`, `T = i32|u32|f32`, `want = Set|Map|string|Result|Option`, `ok = int|f32|f64|bool`.

Guards, hooks, modifiers and facts are `#[derive(Deserialize)]` enums in src/stdlib_surface/lowering.rs, so the
emitter matches enum variants, never strings. They name type shapes and emitter capabilities, never std methods; a gate
caps their variant count (section 4, G6) so the vocabulary cannot become a second name table. Anything a template cannot
say becomes a runtime helper in `std/runtime/{js/wj_core.mjs,go/wj_core.go}` (tested once, used by name in the template)
or a Windjammer default body on the declaration (`fn is_power_of_two(self) -> bool { self.count_ones() == 1 }`), which
every backend and the interpreter then run from source.

### 2.4 The emitter after migration

```rust
// src/codegen/javascript/emit/builtin.rs (shape, all three backends alike)
pub fn method(&mut self, recv: &PExpr, name: &str, args: &[PExpr], res: &Resolution, ty: &Ty) -> PResult<String> {
    let row = self.surface.lookup(CoreType::of(&recv.ty), name)?;   // data keyed by the declaration's name
    match row.rule(Target::Js).select(&self.facts(recv, args, ty)) {
        Rule::Emit(t) => t.render(&self.operands(recv, args, row)?),
        Rule::Assign(t) => self.assign_place(recv, |p| t.render_with_place(p, ..)),
        Rule::Hook(h) => self.hook(h, recv, ty),
        Rule::Unsupported(why) => self.unsupported(why),
    }
}
```

The lookup is a `BTreeMap` keyed by strings read from the declarations; no literal method name appears in Rust.
`CoreType::of` maps a `Ty` variant to the trait it is declared in (a closed enum, deserialized from the trait name).

## 3. Interactions

**ADR-0039 (one way, no back-compat).** The emitters today accept removed spellings (`to_owned`, `as_str`,
`push_str`, `extend_from_slice`, `sort_unstable`, `into_keys`, `values_mut`, `push_back` on Vec, `is_ascii_*`,
`len_utf8`, `substring`, `as_bytes`, `to_vec`, `powi`/`powf`, the `std::math` and `std::strings` free functions). They
are **not migrated as rows**. A table row whose name is a `removed` row of `std_surface.toml` is refused unless it is
`@internal(reason = "...")`: a lowering target the compiler itself produces (`unwrap_or` for `??`, `iter` from a `for`
desugaring), with the desugaring named. Dropping an alias is proven safe by the emit-hash golden (G2): if no accepted
program reaches it, the output is byte-identical. Decision B (type of `len`) is untouched: the narrowing rule reads the
declared `usize`, whatever the surface spelling becomes.

**The explicit re-exports ADR (not yet written).** A module function is keyed by the module that declares it. A function a module
offers through `pub use child::name` or `pub use child::*` (`std/time.wj` re-exports `calendar`) is looked up through the
resolver's explicit re-export, never by searching children. The table gate (G1) resolves every `std::<module>::<fn>` the
corpus calls through the same resolver, so an implicit child lookup fails the gate exactly like an unresolved name fails a program.

**Interpreter.** The interpreter keeps native Rust implementations (it cannot run JS or Go templates), registered by
`(Kind, name)` in `src/interpreter/builtins/*.rs`. It reads the table for everything that is not the implementation:
signature and arity checks, `&mut self` (the `mutating` vs `pure` registration must agree with the declaration), effects,
and Windjammer default bodies (run from source, so a body-defined row needs no native code). Gate G4 requires every
row to have a registration or `@interp(unsupported = reason)`, and every registration to be a row or `@internal`;
this replaces the parity claim in the `std_surface.toml` header, which no test enforces at this head.

**Check-time portability diagnostic (JS json).** `wj check` with a JS, TS or Go target (and `wj build` for those targets,
before emitting) resolves each std call to its row and reports `unsupported` rules at check time:

```
error[WJ0195]: `json::parse` is not available on javascript
  --> src/main.wj:13:16
   = note: std/runtime/js/wj_json.mjs exists but is not wired to the portable lowering (stage 7)
   = help: build for rust, or keep the json code behind a module only the rust target compiles
```

Today the same program fails mid-emission with "unknown associated function `json::parse`" (TARGET_TIERS: the
`json_operations` conformance program, all three portable backends). With no `--target`, nothing is reported. The
diagnostic reads only the row (decided by declaration), so it is identical in the interpreter's `--target` prediction
mode, which warns rather than refuses.

## 4. Gates (each mutation-checked)

| id | gate | mutation that must make it fail |
|---|---|---|
| G1 | tests/std_lowering_table_test.rs: every declaration in `rust_core.wji`, `std/*.wj`, `enumerable.wj` has, per target, a rule or `unsupported` with a non-empty reason; every template parses; every `{name}` is a declared parameter; every `when` list ends unguarded; no `${` (string interpolation) in a template | delete one `@go` from a row; write `{indx}`; empty a reason |
| G2 | tests/golden/portable_emit_hashes.tsv: JS, TS and Go output hash of every conformance program and spec run case; must be byte-identical across every migration stage (re-record only per `golden-rerecord-by-cause`) | change `$sort(` to `$sort2(` in one row |
| G3 | per-row conformance corpus, generated from each row's `@example(src = "...", expect = "...")`: run interpreted, Rust, JS, TS, Go; output equals `expect`; on an `unsupported` target the program must fail at check time with the portability diagnostic carrying the reason | swap a template's operands (`{a}`, `{b}`); mark a working row unsupported |
| G4 | interpreter parity: rows vs registry, both directions; `&mut self` vs `mutating` registration agree | register a method with no row; flip a row to `&mut self` |
| G5 | helper existence: every `$name` / `wjName` a template uses is defined in `wj_core.{mjs,go}` / `wj_std.*`; a helper no row uses fails (shrink-only) | rename one helper in the shim |
| G6 | vocabulary cap: variant counts of `Guard`, `Hook`, `Modifier`, `Fact` are pinned in the test; a new one needs the test edited with a reason | add a `Guard::NameIs(String)` variant |
| G7 | heuristic surface: the migrated file's allowlist entry falls to the stage target; after stage 6 the three builtin files have no entry | add `"push" =>` back to the JS emitter |
| G8 | ADR-0039 cross-check: a row named like a `removed` row needs `@internal(reason)`; every canonical `std_surface.toml` row resolves to one declaration | add `fn to_owned` to `trait String` without `@internal` |

Plus, unchanged at every stage: `tests/conformance/backend_ledger.tsv` (via `scripts/wj/target_tiers.wj check`) and the
rustc gate on `rust_core.wji` (rows added for portable-only methods are rustc-checked like the rest).

## 5. Migration stages

Each stage moves one receiver family for **all three** consumers at once (typer, JS, Go), so a row is complete the day it
lands. Every stage: G2 hashes identical, ledger identical, heuristic total falls by the stated amount (counts from
section 1). Stage 7 is the only one allowed to change the ledger, and only by removing entries.

| stage | scope | builtins.rs | JS | Go | other | total after |
|---|---|---|---|---|---|---|
| 1 | loader, template engine, G1-G6, G8; **Float** rows (incl. Go f32 widen) | -34 | -34 | -34 | | **1,638** |
| 2 | **Integer** rows (wrapping/checked/saturating via `{T.*}` facts; `wrapping_mul` guards `T = i32 / u32`); add `abs_diff`, `isqrt`, `is_power_of_two` declarations | -34 | -31 | -31 | | **1,542** |
| 3 | **char** (new `trait Char` in `rust_core.wji`) and **String**; `parse` via `ok =` guards; removed aliases dropped | -53 | -50 | -48 | | **1,391** |
| 4 | **Option**, **Result**; `@internal` rows for `??` | -27 | -22 | -22 | | **1,320** |
| 5 (done) | **HashMap**, **HashSet**, statics (`Vec::new`, `i64::MAX`, `f64::EPSILON`, `char::from_u32`) as static declarations and `const` items | -33 | -37 | -34 | | **1,216** |
| 6 | **Vec**, **Iterator** (`@adapts`), `collect`/`sum` by want, the `len` narrowing rule, `to_string` hook, `ret_ty` residue | -79 | -72 | -69 | | **996** (amber) |
| 7 | module functions: `fs`, `time` rows on `std/fs.wj`, `std/time.wj`; `math` / `strings` free functions deleted (ADR-0039 R058/R070, after `wj fix` migrates the conformance programs that use them); **json wired** to the existing shims; the portability diagnostic | | | | stdlib.rs -68, lower/call.rs -2 | **926** |
| 8 | enumerable and ordering families from `std/enumerable.wj` (`src/stdlib_surface/enumerable.rs` generated) | | | | -93 | **833** |
| 9 | scanner tightening: tuple-pattern literals and `matches!(x, "a" \| ..)` lists counted (report-only from stage 1, enforced now) | | | | measured at stage 1 | 833 + new baseline |

After stage 6 the three builtin files hold 0 (260, 246, 238 all retired); after stage 8 the portable family holds only
`ty.rs` (15), `lower/call.rs` (9), `lower/fmt.rs` (6). Follow-on outside this plan: the Rust backend's std handling
(`stdlib_method_traits.rs`, `method_call_expression_generation/{finalize,std_idiom_fixups,idiom_lowering,arguments}.rs`,
about 60 in the allowlist) reads the `rust` rule; `ty.rs` reads the trait names. This plan takes the surface from red to
amber; green needs the Rust-backend streams of `convergence-burndown.md` as well.

## 6. Risks

| risk | mitigation |
|---|---|
| the template language grows into a programming language | closed enums, G6 cap; complex forms become a tested runtime helper or a Windjammer default body |
| a behaviour change hides in a "refactor" | G2 byte-identical hashes over the whole corpus per stage; G3 per row on every backend |
| a removed alias is still produced by a desugaring | G2 catches the output change; the fix is an `@internal` row naming the desugaring |
| decorator strings collide with string interpolation (`${`) | G1 rejects `${`; templates use `{..}` only |
| compile-time cost (the build-speed promise) | the table is parsed once per session with `CoreSurface` (already done for the solver); measure `wj build` wall/RSS on the conformance corpus before and after stage 1, budget +2% |
| rows added to `rust_core.wji` for portable-only names break the rustc gate | declare them in the trait where rustc has them (`abs_diff`, `isqrt`, `is_power_of_two` exist on Rust ints); `Char` is a new trait checked the same way |
| interpreter and table diverge | G4 both directions; the interpreter reads arity/mutation from the row |
| ADR-0039 decision B lands mid-migration | the narrowing rule reads `usize` declarations, so B changes declarations, not emitters |

## 7. Work breakdown (one sonnet agent per stage, in order)

Common to every stage: worktree off the integration head; builds and tests through the build-slot wrapper with
`CARGO_BUILD_JOBS=2`; run `cargo test --test all heuristic_surface_test std_lowering_table_test`, the G2 hash test, the
G3 corpus for the stage's rows, `wj run --interpret scripts/wj/target_tiers.wj check`, and `core_surface_rustc_test`;
lower the allowlist with `regenerate_heuristic_baseline` and update the totals line in `convergence-burndown.md`;
record each gate's mutation result in the commit message. Never edit a golden to pass; never raise the allowlist.

**Stage 1 (brief):** *Goal:* the table machinery plus the Float family; heuristic 1,740 -> 1,638.
- New src/stdlib_surface/lowering.rs (layer `support`, add to `MODULE.md` / `docs/MODULE_MAP.toml`): read decorators
  from the `CoreSurface` parse of `rust_core.wji` into `LoweringTable { rows: BTreeMap<(CoreType, String), Row> }`;
  `Row { sig, rules: BTreeMap<Target, Vec<Rule>>, example }`; `Rule`, `Guard`, `Hook`, `Modifier`, `Fact` as serde enums.
- New src/codegen/portable/template.rs: parse and render templates (`{self}`, `{&self}`, `{param[:mod]}`, `{T.fact}`).
- `std/rust_core.wji` `trait Float<T>`: `@js` / `@go` / `@example` on every method the two emitters handle today
  (`sqrt abs floor ceil round trunc fract powi powf sin cos tan asin acos atan exp cbrt ln log10 log2 min max clamp
  signum atan2 hypot is_nan is_infinite is_finite is_sign_negative is_sign_positive to_degrees to_radians mul_add`),
  copying today's output exactly; `@go(widen = "f64")` on the trait.
- `src/codegen/portable/builtins.rs`: delete `float_ret`; `ret_ty` for `Ty::F64 | Ty::F32` reads the instantiated
  signature. `javascript/emit/builtin.rs` and `go/emit/builtin.rs`: `float_method` becomes the row interpreter of 2.4.
- New tests/std_lowering_table_test.rs (G1, G4 for Float, G5, G6, G8) and tests/golden/portable_emit_hashes.tsv
  with its test (G2), recorded on the parent commit first, then required identical on the stage commit. G3 runner over
  the Float rows' `@example`s. Scanner report-only mode for stage 9 (prints, never fails).
- *Acceptance:* G1-G8 pass; each mutation in section 4 fails its gate (run them, list the results); G2 identical; ledger
  identical; `wj build` wall time on the conformance corpus within 2% of the parent.
- *Measured target:* `builtins.rs` 226, JS `builtin.rs` 212, Go `builtin.rs` 204; total **1,638**.

**Stage 2:** Integer. Files: `rust_core.wji` `trait Integer<T>` (add `abs_diff`, `isqrt`, `is_power_of_two`;
rustc gate), `builtins.rs` `int_ret`, both `int_method`s; facts `T.min/max/zero/one/chk/bits/signed/wide/go`. Target
`builtins.rs` 192, JS 181, Go 173; total **1,542**.

**Stage 3:** char + String. Files: new `trait Char` in `rust_core.wji`, `trait String`; `char_ret`, `str_ret`,
`str_arg_want`, both `char_method` / `str_method`; `parse` with `ok =` guards; `push` / `clear` via `assign`; removed
aliases deleted (G2 proves unreachable; else `@internal`). Target `builtins.rs` 139, JS 131, Go 125; total **1,391**.

**Stage 4:** Option + Result. Files: `trait Option<T>`, `trait Result<T, E>`; `option_ret`, `result_ret`, both
`*_arg_want`, both `option/opt_method`, `result/res_method`; `@internal` rows for `??` and `?` desugarings. Target
`builtins.rs` 112, JS 109, Go 103; total **1,320**.

**Stage 5:** maps, sets, statics. Files: `trait HashMap/BTreeMap/HashSet/BTreeSet` (bound `K: Key`), static `fn new`,
`with_capacity`, `from`, `const MAX/MIN/EPSILON/...` on the numeric traits; `map_ret`, `set_ret`, `static_ret`,
`float_const`, both `map_method`, `set_method`, `builtin_static`. Target `builtins.rs` 79, JS 72, Go 69; total **1,216**.

**Stage 6:** Vec + Iterator. Files: `trait Vec<T>` (`@adapts(Iterator)`), `trait Iterator<T>`, `collect` / `sum` /
`product` with `@result_from_context`; `vec_ret`, `iter_ret`, `ret_ty`, `vec_arg_want`, `iter_arg_want`, `stores_args`
(owned params), both `vec_method` / `iter_method`, `sum_op`, `collect`, Go `mutating`, the `len` narrowing and
`to_string` dispatch. Target: the three files absent from `[match_arm_names]` and `[name_matches]`; total **996**.

**Stage 7:** module functions + json + the portability diagnostic. Files: decorators on `std/fs.wj`, `std/time.wj`, `std/json.wj`
(wire `wj_json.mjs` / `wj_json.go`), file-level `unsupported` on the Rust-only modules; delete `stdlib.rs`;
`lower/call.rs` path calls read the table; `src/check/` portability pass and `docs/DIAGNOSTICS.md` row; migrate the
conformance programs off `math::` / `strings::` free functions with `wj fix` first. Ledger: `json_operations` leaves
the JS/TS/Go ledger. Target total **926**.

**Stage 8:** enumerable + ordering. Files: decorators on `std/enumerable.wj` and the `Ordering` declarations;
`src/stdlib_surface/enumerable.rs` generated (gate: fresh); `portable/{enumerable,ordering}.rs`,
`{javascript,go}/emit/{enumerable,ordering}.rs` become row interpreters. Target total **833**.

**Stage 9:** enforce the scanner extension (tuple patterns, `matches!` lists) with the baseline measured in stage 1's
report; the files of this plan contribute 0.
