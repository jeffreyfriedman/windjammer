# Examples

Status: these programs are not built by CI. They may lag the compiler; the language-feature examples in
`syntax_tests/` and `tests/` are run with `wj test` (see below), everything else is best-effort.

## Contents
- `syntax_tests/` - numbered single-feature programs (package with tests).
- `tests/` - small test-framework examples.
- `hello_world/`, `pipe_operator/`, `cli_tool/`, `http_server/`, `file_server/` - small programs.
- `wjfind/` (file search), `wschat/` (WebSocket chat), `taskflow/` (REST API): larger dogfooding programs.
- `wasm_hello/` - WebAssembly "hello" (`wj build ... --target wasm`).
- `optimization_validation/` - inputs for the optimizer assessment.
- Single files in this directory (`async_patterns.wj`, `traits.wj`, `macros.wj`, ...): feature demos, not verified.

## Running

```bash
wj run examples/hello_world/main.wj      # adjust to the file in that directory
wj test examples/syntax_tests
wj test examples/tests
```
