//! A `use std::fs` brings the runtime module's exported types into scope (`Metadata`, `DirEntry`); a program that
//! never names them would draw an unused-import warning from rustc on every build.

use super::stdlib_method_traits::runtime_std_module_for_type;
use super::CodeGenerator;

impl CodeGenerator<'_> {
    /// Drop `use windjammer_runtime::<module>::<Type>;` lines whose scanned runtime type the generated `body` never names.
    pub(in crate::codegen::rust) fn prune_unused_runtime_type_imports(imports: &str, body: &str) -> String {
        const RUNTIME: &str = "use windjammer_runtime::";
        let named_in_body = |ident: &str| {
            body.match_indices(ident).any(|(at, _)| {
                let before = body[..at].chars().next_back();
                let after = body[at + ident.len()..].chars().next();
                let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
                !word(before) && !word(after)
            })
        };
        let unused = |line: &str| {
            let Some(path) = line.trim().strip_prefix(RUNTIME).and_then(|r| r.strip_suffix(';')) else { return false };
            let Some((_, ident)) = path.rsplit_once("::") else { return false };
            ident.chars().next().is_some_and(char::is_uppercase)
                && ident.chars().all(|c| c.is_alphanumeric() || c == '_')
                && runtime_std_module_for_type(ident).is_some()
                && !named_in_body(ident)
        };
        let mut kept: Vec<&str> = imports.lines().filter(|l| !unused(l)).collect();
        if imports.ends_with('\n') {
            kept.push("");
        }
        kept.join("\n")
    }
}
