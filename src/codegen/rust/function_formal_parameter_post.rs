//! Post-processing of emitted formal parameters: callables and thread captures.

use crate::analyzer::*;
use crate::parser::*;
use std::collections::HashSet;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// A `fn(A) -> B` formal accepts any callable, including capturing closures. A Rust `fn`
    /// pointer cannot hold a capture, so non-extern formals are emitted as `impl Fn(A) -> B`.
    pub(in crate::codegen::rust) fn collect_formal_parameter_strings(
        &mut self,
        analyzed: &AnalyzedFunction<'ast>,
        func: &FunctionDecl<'ast>,
        needs_lifetime: bool,
        unused_params: &HashSet<String>,
    ) -> Vec<String> {
        self.idiom.non_escaping_closure_names = super::closure_escape::non_escaping_closure_names(&func.body);
        let raw = self.collect_additional_formal_parameter_strings(
            analyzed,
            func,
            needs_lifetime,
            unused_params,
        );
        if func.is_extern {
            return raw;
        }
        let raw = self.upgrade_std_trait_self_receiver(func, analyzed, raw);
        func.parameters
            .iter()
            .zip(raw)
            .map(|(param, text)| {
                // Captured by a closure given to `spawn`: the thread owns it, a `&T` formal cannot be.
                if param.name != "self"
                    && crate::analyzer::param_captured_by_spawned_closure(
                        &param.name,
                        func.body.as_slice(),
                    )
                {
                    let marker = format!("{}: &", param.name);
                    if let Some(i) = text.find(&marker) {
                        let after = &text[i + marker.len()..];
                        if !after.starts_with("mut ") {
                            self.inferred_borrowed_params.remove(&param.name);
                            self.emitted_rust_ref_formals.remove(&param.name);
                            return format!("{}{}: {}", &text[..i], param.name, after);
                        }
                    }
                }
                if matches!(param.type_, Type::FunctionPointer { .. }) {
                    let marker = format!("{}: fn(", param.name);
                    if let Some(i) = text.find(&marker) {
                        let at = i + param.name.len() + 2;
                        return format!("{}impl Fn({}", &text[..at], &text[at + 3..]);
                    }
                }
                text
            })
            .collect()
    }

    /// Std traits that the program does not declare: `Iterator::next` (and friends) take `&mut self`, and any
    /// other method of such a trait that mutates `self` must too (a `&self` receiver cannot assign).
    fn upgrade_std_trait_self_receiver(
        &mut self,
        func: &FunctionDecl<'ast>,
        analyzed: &AnalyzedFunction<'ast>,
        mut raw: Vec<String>,
    ) -> Vec<String> {
        if !self.in_trait_impl {
            return raw;
        }
        let Some(tr) = self
            .current_trait_impl_name
            .as_deref()
            .map(|t| t.rsplit("::").next().unwrap_or(t).to_string())
        else {
            return raw;
        };
        let std_trait = matches!(
            tr.as_str(),
            "Iterator" | "DoubleEndedIterator" | "Write" | "Read" | "Hasher" | "Extend" | "Drop"
                | "AddAssign" | "SubAssign" | "MulAssign" | "DivAssign" | "IndexMut" | "DerefMut"
        ) && !self.analyzed_trait_methods.contains_key(&tr);
        let always_mut = matches!(
            (tr.as_str(), func.name.as_str()),
            ("Iterator", "next") | ("DoubleEndedIterator", "next_back")
        );
        if !std_trait || !(always_mut || self.function_modifies_self(&analyzed.decl)) {
            return raw;
        }
        for (param, text) in func.parameters.iter().zip(raw.iter_mut()) {
            if param.name == "self" && (text == "&self" || text == "self" || text == "mut self") {
                *text = "&mut self".to_string();
                self.inferred_borrowed_params.remove("self");
                self.inferred_mut_borrowed_params.insert("self".to_string());
            }
        }
        raw
    }

    /// `impl PartialEq/PartialOrd/Ord for T`: every non-self formal is `&Self` in the std trait.
    pub(super) fn std_comparison_formal_type(&self, type_str: String) -> String {
        let std_comparison_trait = self.current_trait_impl_name.as_deref().is_some_and(|t| {
            matches!(t.rsplit("::").next().unwrap_or(t), "PartialEq" | "PartialOrd" | "Ord")
        });
        if std_comparison_trait && !type_str.starts_with('&') {
            format!("&{type_str}")
        } else {
            type_str
        }
    }
}
