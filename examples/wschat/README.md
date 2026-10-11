# wschat

A WebSocket chat server written in Windjammer: connections, rooms, direct messages, presence, heartbeats, rate limiting and message
storage, split across the modules in `src/` (entry point `src/main.wj`), with a load-test program in `tests/load_test.wj`.

Status: `tests/examples_gate_test.rs` parses and analyzes every file here (`examples/EXAMPLES.toml`). It needs network access and the
stdlib HTTP/WebSocket support, so it is not run in CI, and this README makes no performance claims.
