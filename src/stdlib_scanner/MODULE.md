# stdlib_scanner

## Purpose
Reads `windjammer-runtime/src/*.rs` and the `std/*.wj` stubs to build the standard-library `SignatureRegistry`
(parameter ownership, return types, exported types, runtime module stems). The parent file `src/stdlib_scanner.rs`
holds the scanner; submodules hold self-contained helpers split out to keep the file under the length ratchet.

## Inputs / outputs
In: the runtime crate source (resolved without depending on the current directory). Out: scanned
`FunctionSignature`s registered into a `SignatureRegistry`.

## Invariants
- Scanning is deterministic and text-based; a line it cannot parse is skipped, never guessed.
- A return type naming a generic runtime struct (`arena::Handle<T>`) is erased to `Type::Infer`, because the scanner
  keeps only the base name and a bare `Handle` is not a valid Rust type (`generic_returns.rs`).

## Owns / never decides
Owns: signature extraction from runtime source. Never decides ownership inference or call-site coercion.

## Allowed imports
Same as the parent module (`analyzer` types, `parser::Type`); submodules reach them through `super::`.

## Layout
`generic_returns.rs` generic-struct return erasure (ADR-0025 `std::arena`).
