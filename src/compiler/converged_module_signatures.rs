//! After multipass convergence, make every module-qualified function key (`domain::text::join`)
//! hold the file's converged signature, never the declaration stub (convergence plan C3).
//!
//! Declaration stubs register each function under its bare name and its module-qualified key with
//! all-owned formals. Convergence refines the bare key per file; the qualified key kept the stub, so
//! a call written `crate::domain::text::join(..)` resolved to the stub and got no `&` for `&str`
//! formals. The converged signature of a function is the one its *own* file analysis produced, so
//! that is the one registered under the file's qualified key (homonyms in other files cannot collide).

use crate::analyzer::SignatureRegistry;
use crate::parser::ast::core::{Item, Program};

/// Register `file`'s converged free-function signatures under their module-qualified keys.
/// Every key registered here is authoritative: other files' analyses carry the stub for it and must
/// not overwrite it (`authoritative` collects the keys for that check).
pub(crate) fn register_converged_module_qualified_signatures(
    registry: &mut SignatureRegistry,
    file_module: &[String],
    program: &Program<'_>,
    file_analysis: &SignatureRegistry,
    authoritative: &mut std::collections::HashSet<String>,
) {
    register_items(registry, file_module, &program.items, file_analysis, authoritative);
}

fn register_items(
    registry: &mut SignatureRegistry,
    prefix: &[String],
    items: &[Item<'_>],
    file_analysis: &SignatureRegistry,
    authoritative: &mut std::collections::HashSet<String>,
) {
    for item in items {
        match item {
            Item::Function { decl, .. } if decl.parent_type.is_none() && !prefix.is_empty() => {
                if let Some(sig) = file_analysis.signatures.get(&decl.name) {
                    let key = format!("{}::{}", prefix.join("::"), decl.name);
                    let mut sig = sig.clone();
                    sig.name = key.clone();
                    authoritative.insert(key.clone());
                    registry.signatures.insert(key, sig);
                }
            }
            Item::Mod { name, items, .. } => {
                let mut next = prefix.to_vec();
                next.push(name.clone());
                register_items(registry, &next, items, file_analysis, authoritative);
            }
            _ => {}
        }
    }
}
