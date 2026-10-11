//! Go backend over the portable typed IR.
//!
//! Output is a ready-to-run module directory: `main.go`, `wj_core.go` (runtime), `go.mod`.
//! See `MODULE.md`.

mod emit;

use crate::codegen::backend::{CodegenBackend, CodegenConfig, CodegenOutput, Target};
use crate::codegen::portable::{self, flatten};
use crate::parser::Program;
use anyhow::{anyhow, Result};

/// Runtime support file shipped next to every generated program.
pub const RUNTIME_SOURCE: &str = include_str!("../../../std/runtime/go/wj_core.go");
/// Portable `std::{math,strings,fs,time}` shims.
pub const STD_SOURCE: &str = include_str!("../../../std/runtime/go/wj_std.go");

pub struct GoBackend;

impl GoBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GoBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CodegenBackend for GoBackend {
    fn name(&self) -> &str {
        "Go"
    }

    fn target(&self) -> Target {
        Target::Go
    }

    fn generate(&self, program: &Program, _config: &CodegenConfig) -> Result<CodegenOutput> {
        let ctx = crate::ir::analyze_and_lower(program)?;
        let mut m = portable::lower::lower_program(program, Some(&ctx.registry)).map_err(|e| anyhow!("{e}"))?;
        flatten::flatten_module(&mut m);
        let src = emit::emit_module(&m).map_err(|e| anyhow!("{e}"))?;
        Ok(CodegenOutput::new(src, "go".to_string()))
    }

    fn generate_additional_files(&self, _program: &Program, _config: &CodegenConfig) -> Vec<(String, String)> {
        vec![
            ("wj_core.go".to_string(), RUNTIME_SOURCE.to_string()),
            ("wj_std.go".to_string(), STD_SOURCE.to_string()),
            ("go.mod".to_string(), "module windjammer-program\n\ngo 1.22\n".to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_identity() {
        let b = GoBackend::new();
        assert_eq!(b.name(), "Go");
        assert_eq!(b.target(), Target::Go);
    }

    #[test]
    fn emits_runtime_and_go_mod() {
        let files = GoBackend::new().generate_additional_files(&Program::new(vec![]), &CodegenConfig::default());
        assert!(files.iter().any(|(n, _)| n == "go.mod"));
        assert!(files.iter().any(|(n, c)| n == "wj_core.go" && c.contains("package main")));
    }
}
