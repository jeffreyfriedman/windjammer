# Interpreter: threads, channels, `fs::read_dir`, `fs::write_string` are unsupported

Reported loudly (exit 2, "unsupported in the interpreter"), never silently, so this is a documented limit rather than a bug. It decides
the `wj-interp` column of the `bench/agent-dx` matrix: t042-t046 (threads/channels/Mutex) and t054 (directory walk) cannot run
interpreted. `fs::write` works where `fs::write_string` does not (t051 uses `fs::write`).
An agent only learns this at run time, one feature at a time; the first-error-to-fix metric captures that cost.
