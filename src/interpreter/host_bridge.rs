//! Host bridge for `extern fn` (ADR-0018): calls into a cached, compiled-once dynamic library.

use super::engine::Interpreter;

impl Interpreter {
    /// True when a loaded host library provides `extern fn name`.
    pub(crate) fn has_host_fn(&self, _name: &str) -> bool {
        false
    }
}
