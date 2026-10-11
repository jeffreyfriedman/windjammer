# TaskFlow

A task-management REST API written in Windjammer (auth, projects, tasks, middleware, migrations).
It is a large dogfooding program for the standard library (`std::http`, `std::db`, `std::jwt`, ...).

Status: not part of CI; it is not guaranteed to compile with the current compiler.
Build attempt: `wj build examples/taskflow/windjammer/src/main.wj` (see `wj.toml` in that directory).
