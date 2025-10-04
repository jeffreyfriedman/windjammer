fix: Critical codegen bugs discovered through WASM example

Fixed 4 critical compiler bugs by working through complex wasm_game example:

## 1. Operator Precedence Bug
- Binary expressions weren't respecting operator precedence
- Example: `a + b % c` was generated as-is, but `%` binds tighter than `+`
- Added `op_precedence()` to wrap lower-precedence subexpressions

## 2. Missing Glob Imports
- `use wasm_bindgen.prelude` generated `use wasm_bindgen::prelude;` (wrong)
- Fixed to generate `use wasm_bindgen::prelude::*;` for proper imports

## 3. Impl Block Decorators
- `@wasm_bindgen` on impl blocks wasn't being parsed or generated
- Added decorator support to ImplBlock AST node
- Parser now captures decorators before impl keyword

## 4. Missing `pub` in WASM Impl Blocks
- Functions in `#[wasm_bindgen]` impl must be `pub` to export to JS
- Added `in_wasm_bindgen_impl` flag to track decorator context
- Codegen now prepends `pub` when in WASM impl block

## Testing
- ✅ All 9 compiler tests passing
- ✅ wasm_hello example works (simple WASM functions)
- ✅ wasm_game example works (Conway's Game of Life at 60 FPS)

## Files Changed
- src/codegen.rs: +175 lines (precedence, imports, pub, tracking)
- src/parser.rs: +247 lines (impl decorators, Copy type fixes)
- src/analyzer.rs: +36 lines (Copy type handling)
- src/lexer.rs: +30 lines (character literals)
- tests/*: Updated expectations for correct Copy type behavior
- examples/wasm_game/*: Working browser demo

This demonstrates the value of complex real-world examples in language development.

