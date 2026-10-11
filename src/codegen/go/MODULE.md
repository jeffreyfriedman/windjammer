# go (Go backend)

**Purpose.** Emit a ready-to-run Go module from the portable IR: `main.go`, `wj_core.go` (runtime), `go.mod`.

**Inputs/outputs.** `Program` -> (`portable::lower` -> `flatten`) -> `emit::emit_module` -> source text.
`CodegenBackend::generate_additional_files` supplies `wj_core.go` (embedded from `std/runtime/go/wj_core.go`) and `go.mod`.

**Representation contract.**
- ints: exact Go widths (`usize` = `uint`, `isize` = `int`); `+ - * / %` go through generic checked helpers and panic on overflow
  (Rust debug semantics); `wrapping_*` use native Go wrap-around; shifts check the count.
- `char` = `Char` (a defined rune type), `Vec` = slice, `HashMap` = map, `HashSet` = `map[T]struct{}`.
- `Option[T]`, `Result[T, E]`, `TupleN[...]` are runtime generics; enums are an interface plus one struct per variant.
- Struct fields are exported (`X`) so reflection-based `wjClone`/`wjDbg` work. Methods keep lower-case names.
- `&mut` params are pointers (`*T`); maps/sets are reference types and are passed as-is.
- Panics (explicit, overflow, index, divide by zero) exit with status 101.

**Intrinsics.** `println`/`stderr.println`/`assert*`/`panic`/`todo`/`unreachable`/`debug` need no code here: the portable layer lowers them to `PK::Print`/`PK::Panic`
(`wjPrint`/`wjEprint`/`wjPanic`). Format specs: `wjPad`, `wjFixed`, `wjRadix`, `wjDbg`; unsupported specs are `Unsupported` errors.

**Invariants.** No imports in `main.go` (everything goes through `wj*` helpers); every declared local gets `_ = x`
(Go rejects unused variables); loops that need it get labels; non-terminating non-unit functions end in `panic`.

**Owns / never decides.** Owns Go spelling. Never decides typing or ownership (portable layer does).

**Entry points.** `GoBackend`, `emit::emit_module`. CLI: `wj build --target go FILE -o DIR`, then `cd DIR && go run .`.

**Known limitations.** Generic methods (Go methods cannot declare type parameters), trait objects over enums,
closures that mutate captured `&mut` Vec parameters, struct map keys. These are `Unsupported` errors, not wrong code.

**Testing.** `cargo test --release --test all backend_conformance`; needs `go` (skips are loud; `WJ_REQUIRE_TOOLCHAINS=1` fails them).
