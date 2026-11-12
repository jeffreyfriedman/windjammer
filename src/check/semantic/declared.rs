//! The declarations the semantic check reads types from: the `String` methods of `std/rust_core.wji` (through
//! `typing::CoreSurface`) and the return types of the functions of the std modules a project imports
//! (`std/strings.wj`: `fn chars(s: string) -> Vec<char>`). Built once per check run, with the index.

use crate::parser::{Item, Type};
use crate::typing::core_surface::CoreSurface;
use std::collections::HashMap;

pub struct Declared {
    core: CoreSurface,
    /// (std module, function) -> declared return type.
    returns: HashMap<(String, String), Type>,
}

impl Declared {
    /// Load the surface and the declarations of `modules` (std module names such as `strings`).
    pub fn load<'a>(modules: impl Iterator<Item = &'a str>) -> Self {
        let mut returns = HashMap::new();
        let asset = crate::embedded_assets::Asset::StdWj;
        for module in modules {
            let text = crate::embedded_assets::text(asset, &format!("{module}.wj"))
                .or_else(|| crate::embedded_assets::text(asset, &format!("{module}/mod.wj")));
            let Some(text) = text else { continue };
            let tokens = crate::lexer::Lexer::new(text).tokenize_with_locations();
            let Ok(program) = crate::parser_impl::Parser::new(tokens).parse() else { continue };
            for item in &program.items {
                if let Item::Function { decl, .. } = item {
                    if let Some(ret) = &decl.return_type {
                        returns.insert((module.to_string(), decl.name.clone()), ret.clone());
                    }
                }
            }
        }
        Self { core: CoreSurface::parse(), returns }
    }

    /// The declared return type of `std::<module>::<function>`.
    pub fn std_return(&self, module: &str, function: &str) -> Option<&Type> {
        self.returns.get(&(module.to_string(), function.to_string()))
    }

    /// The declared result of `text.method()` on a `String` receiver.
    pub fn string_method_return(&self, method: &str) -> Option<Type> {
        self.core.method_signature(&Type::String, method, &[])?.ret
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn std_returns_come_from_the_std_declaration() {
        let d = Declared::load(["strings"].into_iter());
        assert_eq!(d.std_return("strings", "chars"), Some(&Type::Vec(Box::new(Type::Custom("char".into())))));
        assert_eq!(d.std_return("strings", "no_such_function"), None);
        assert_eq!(d.std_return("fs", "chars"), None, "modules that were not loaded declare nothing");
    }

    #[test]
    fn string_chars_is_an_iterator_of_char() {
        let d = Declared::load(std::iter::empty());
        let ret = d.string_method_return("chars").expect("declared in std/rust_core.wji");
        assert_eq!(super::super::elements::element_of_type(&ret), Some(super::super::rules::Kind::Char));
    }
}
