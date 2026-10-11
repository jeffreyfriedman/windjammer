# taskflow

A REST API (users, projects, tasks, authentication, rate limiting) written in Windjammer, in `windjammer/`. It is a dogfooding program, not a
benchmark: `tests/examples_gate_test.rs` parses and analyzes every file (`examples/EXAMPLES.toml`); it needs a database and an HTTP stack, so it
is not run in CI.
