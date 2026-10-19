# `wj build`: an incremental rebuild can emit different call-site borrows than a clean build

Seen on a multi-module app (about 20 files, `store.wj` defines `fn tasks_of(self, project_id: int, filter: TaskFilter)`,
`tasks.wj` calls `store.tasks_of(project_id, f)`):

1. clean `wj build --release src` emits `store.tasks_of(project_id, &f)` and the build passes;
2. after editing only an unrelated file (the HTTP adapter), `wj build --release src` re-emits `tasks.rs` with
   `store.tasks_of(project_id, f)` (rustc `E0308: expected &TaskFilter`), and the build fails;
3. `rm -rf build .wj-cache` and rebuilding passes again.

Not reduced to a minimal program yet. The expected invariant: a rebuild after touching file X must produce byte-identical
Rust for files whose inputs did not change (the ownership convergence passes should reach the same fixed point).
