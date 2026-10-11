# Examples

Status: every program here is exercised by CI. `examples/EXAMPLES.toml` assigns each `.wj` file a mode
(`run`: interpreter output equals its hand-written `// expect:` lines; `test`: `wj test --interpret` passes; `check`: parses and passes the
analyzer, with a stated reason it is not run), and `tests/examples_gate_test.rs` fails on an example that is not listed or does not pass its mode.
An example that fits no mode is deleted. The verified idiom corpus is [verified/](verified/README.md) (checked by `tests/verified_examples_test.rs`).

## Contents
- `syntax_tests/` - numbered single-feature programs (package with tests).
- `tests/` - small test-framework examples.
- `hello_world/`, `pipe_operator/`, `cli_tool/`, `http_server/`, `file_server/` - small programs.
- `wjfind/` (file search), `wschat/` (WebSocket chat), `taskflow/` (REST API): larger dogfooding programs.
- `wasm_hello/` - WebAssembly "hello" (`wj build ... --target wasm`).
- `optimization_validation/` - inputs for the optimizer assessment.
- Single files in this directory (`traits.wj`, `testing_examples.wj`, ...): feature demos, checked by the gate.

## Running

Run from outside the compiler checkout (see [known issue](../docs/known-issues/wj-test-inside-compiler-checkout.md)).

```bash
wj run examples/hello_world/main.wj      # adjust to the file in that directory
wj test examples/syntax_tests
wj test examples/tests
```
