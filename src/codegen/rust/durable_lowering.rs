//! `durable fn` in the Rust backend (ADR-0021 E4b): the body becomes the closure of `windjammer_runtime::durable::run` and every
//! call at a step position is wrapped in `windjammer_runtime::durable::step`. The step positions are facts from
//! `ir::effect_facts::durable`, handed over in `CompilerConfig::durable_steps`; the code generator decides nothing about them.

use crate::ir::effect_facts::durable::step_callee;
use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// `code`, the generated text of `expr`, inside a step wrapper when `expr` is a journaled step of a durable function.
    pub(in crate::codegen::rust) fn wrap_durable_step(&self, expr: &Expression<'ast>, code: String) -> String {
        if self.config.durable_steps.is_empty() {
            return code;
        }
        let (Expression::Call { location, .. } | Expression::MethodCall { location, .. } | Expression::MacroInvocation { location, .. }) = expr else {
            return code;
        };
        let Some(loc) = location else { return code };
        let file = std::fs::canonicalize(&loc.file).unwrap_or_else(|_| loc.file.clone()).to_string_lossy().to_string();
        let key = (file, loc.line, loc.column);
        if !self.config.durable_steps.contains(&key) {
            return code;
        }
        // a step that only reads is journaled lazily (flushed before the next write outside the process)
        let step = if self.config.durable_read_steps.contains(&key) { "step_read" } else { "step" };
        format!("windjammer_runtime::durable::{step}({:?}, || {})", step_callee(expr), code)
    }

    /// The body of a `durable fn` run as a journaled function.
    pub(in crate::codegen::rust) fn wrap_durable_body(&self, func: &FunctionDecl<'ast>, body_code: String) -> String {
        let args: Vec<String> = func.parameters.iter().map(|p| format!("windjammer_runtime::durable::to_json(&{})", p.name)).collect();
        let ret = func.return_type.as_ref().map(|t| self.type_to_rust(t)).unwrap_or_else(|| "()".to_string());
        let pad = self.indent();
        format!(
            "{pad}windjammer_runtime::durable::run({:?}, vec![{}], move || -> {ret} {{\n{body_code}{pad}}})\n",
            func.name,
            args.join(", ")
        )
    }
}
