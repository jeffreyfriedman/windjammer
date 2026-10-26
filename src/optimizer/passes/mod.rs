//! Individual passes. Each reads a parsed file and reports span edits.

pub mod clone_elision;
pub mod dead_helpers;
pub mod inline_policy;
pub mod macro_arg_clone;
pub mod same_type_cast;
pub mod scratch_reuse;
pub mod string_append;
pub mod with_capacity;

use super::config::OptimizationConfig;
use super::edit::{Edit, SourceIndex};

pub struct Ctx<'a> {
    pub file: &'a syn::File,
    pub idx: &'a SourceIndex<'a>,
    pub config: &'a OptimizationConfig,
}

impl Ctx<'_> {
    pub fn edit(&self, span_start: proc_macro2::LineColumn, span_end: proc_macro2::LineColumn, text: String, pass: &'static str) -> Option<Edit> {
        Some(Edit { start: self.idx.offset(span_start)?, end: self.idx.offset(span_end)?, text, pass })
    }
}

/// Collect edits from all enabled passes.
pub fn collect(ctx: &Ctx) -> Vec<Edit> {
    let c = ctx.config;
    let mut edits = Vec::new();
    if c.temp_clone || c.copy_clone {
        edits.extend(clone_elision::collect(ctx));
    }
    if c.macro_arg_clone {
        edits.extend(macro_arg_clone::collect(ctx));
    }
    if c.same_type_cast {
        edits.extend(same_type_cast::collect(ctx));
    }
    if c.with_capacity {
        edits.extend(with_capacity::collect(ctx));
    }
    if c.string_append {
        edits.extend(string_append::collect(ctx));
    }
    if c.inline_policy {
        edits.extend(inline_policy::collect(ctx));
    }
    if c.scratch_reuse {
        edits.extend(scratch_reuse::collect(ctx));
    }
    if c.dead_helpers {
        edits.extend(dead_helpers::collect(ctx));
    }
    edits
}
