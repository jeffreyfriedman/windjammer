//! Entry points for hosts outside `src/` that embed the interpreter (the browser playground, `crates/windjammer-playground`):
//! no `wj` command calls them, so the dead-capability gate lists this file under `[library_api]` in docs/MODULE_MAP.toml.

use super::Interpreter;

impl Interpreter {
    /// Bound recursion by stack bytes from the caller's frame: for hosts that run on their own stack of a known size
    /// (the browser playground's wasm stack) instead of `runner::with_big_stack`.
    pub fn guard_stack_bytes(&mut self, budget: usize) {
        let here = 0u8;
        self.stack_guard = Some((&here as *const u8 as usize, budget));
    }

    /// Run tasks as nested calls on this thread instead of OS threads (always so on wasm32; see `sched.rs`).
    pub fn set_one_thread_tasks(&mut self, on: bool) {
        self.sched.one_thread = on;
    }
}
