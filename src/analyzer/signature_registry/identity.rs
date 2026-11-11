//! Signature lookups by identity (C3): a key the declarations name, never a search over homonyms.

use super::{FunctionSignature, SignatureRegistry};

impl SignatureRegistry {
    /// Lexical scope (C3): inside module `m`, a bare `f` is `m::f` when `m` defines it. Returns that
    /// module-qualified key when it is registered on this layer or a layer below it.
    pub fn scoped_key(&self, module_path: &[String], name: &str) -> Option<String> {
        if module_path.is_empty() || name.contains("::") {
            return None;
        }
        let key = format!("{}::{name}", module_path.join("::"));
        self.has_key_in_chain(&key).then_some(key)
    }

    fn has_key_in_chain(&self, key: &str) -> bool {
        self.layers().any(|reg| reg.signatures.contains_key(key))
    }

    /// The free function an imported path names (`usecases::list_items`, from `use crate::usecases::list_items`):
    /// the key it is registered under (with or without `crate::`) and its signature. C3 identity by
    /// declaration: a method under the key is not a free function, and no other key is searched.
    pub fn imported_free_function(&self, path: &str) -> Option<(String, &FunctionSignature)> {
        [path.to_string(), format!("crate::{path}")].into_iter().find_map(|key| {
            let sig = self.layers().find_map(|reg| reg.signatures.get(&key))?;
            (!sig.has_self_receiver).then_some((key, sig))
        })
    }

    /// True when the exact key `name` (on this layer or a layer below it) holds a method (`self` receiver):
    /// a trait or impl method registered under its bare name.
    pub fn key_holds_method(&self, name: &str) -> bool {
        self.layers().find_map(|reg| reg.signatures.get(name)).is_some_and(|sig| sig.has_self_receiver)
    }
}
