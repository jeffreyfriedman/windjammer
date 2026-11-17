//! Alias registration from a file registry that has been detached from the shared library registry
//! (library multipass, Q-MEM-1): the trait-key facts are taken while the layers are attached, then the file
//! layer is detached so the library registry can be written in place instead of copied per file.

use super::*;

impl SignatureRegistry {
    /// Trait-key facts of every key on this layer, answered through the layers under it too:
    /// `(is_trait_method_key, is_trait_impl_method_key)`. Lets a caller detach those layers first and
    /// still register aliases exactly as [`Self::register_module_aliases`] would.
    pub fn trait_key_flags_of_own_signatures(&self) -> HashMap<String, (bool, bool)> {
        self.signatures
            .keys()
            .map(|name| {
                (
                    name.clone(),
                    (self.is_trait_method_key(name), self.is_trait_impl_method_key(name)),
                )
            })
            .collect()
    }

    /// [`Self::register_module_aliases`] with the trait-key facts of `source` given (from
    /// [`Self::trait_key_flags_of_own_signatures`], taken before `source` was detached).
    pub fn register_module_aliases_with_trait_flags(
        &mut self,
        source: &SignatureRegistry,
        flags: &HashMap<String, (bool, bool)>,
        file_stem: &str,
        module_path: &str,
    ) {
        if file_stem.is_empty() {
            return;
        }
        for (name, sig) in &source.signatures {
            let (is_trait, is_trait_impl) = flags.get(name).copied().unwrap_or((false, false));
            if !name.contains("::") {
                let aliased = format!("{}::{}", file_stem, name);
                if is_trait {
                    self.record_trait_method_key(aliased.clone());
                }
                if is_trait_impl {
                    self.record_trait_impl_method_key(aliased.clone(), source.impl_method_traits.get(name).map(String::as_str));
                }
                // WJ `std/strings.wj` declaration stubs must not clobber scanned
                // runtime AsRef/`&str` under `strings::fn` (P3.524).
                if !self.runtime_strings_alias_would_clobber(&aliased, sig) {
                    self.add_function(aliased, sig.clone());
                }
            }
            if !module_path.is_empty() && !name.starts_with(&format!("{}::", module_path)) {
                let aliased = format!("{}::{}", module_path, name);
                if is_trait {
                    self.record_trait_method_key(aliased.clone());
                }
                if is_trait_impl {
                    self.record_trait_impl_method_key(aliased.clone(), source.impl_method_traits.get(name).map(String::as_str));
                }
                if !self.runtime_strings_alias_would_clobber(&aliased, sig) {
                    self.add_function(aliased, sig.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Type;

    fn plain_sig(name: &str) -> FunctionSignature {
        FunctionSignature {
            name: name.into(),
            param_types: vec![Type::String],
            formal_param_types: vec![Type::String],
            param_ownership: vec![OwnershipMode::Borrowed],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        }
    }

    /// Q-MEM-1: the library build detaches a file's registry from the shared global before merging it (so the
    /// global is updated in place). Aliases registered from the detached layer with the trait-key facts taken
    /// beforehand must equal the ones registered from the attached layer.
    #[test]
    fn detached_alias_registration_matches_the_attached_one() {
        let mut shared = SignatureRegistry::empty();
        shared.record_trait_method_key("Unit::step".into());
        shared.record_trait_impl_method_key("Unit::step".into(), None);
        let shared = std::sync::Arc::new(shared);
        let mut file = SignatureRegistry::layered(std::sync::Arc::clone(&shared));
        file.add_function("Unit::step".into(), plain_sig("Unit::step"));
        file.add_function("helper".into(), plain_sig("helper"));

        let mut attached = SignatureRegistry::empty();
        attached.register_module_aliases(&file, "unit", "world::unit");

        let flags = file.trait_key_flags_of_own_signatures();
        assert_eq!(flags.get("Unit::step"), Some(&(true, true)), "facts come through the shared layer");
        file.detach_shared_layers();
        assert!(!file.is_trait_method_key("Unit::step"), "detached layer no longer reads the global");
        let mut detached = SignatureRegistry::empty();
        detached.register_module_aliases_with_trait_flags(&file, &flags, "unit", "world::unit");

        assert_eq!(
            attached.signatures.keys().collect::<Vec<_>>(),
            detached.signatures.keys().collect::<Vec<_>>()
        );
        for key in ["world::unit::Unit::step", "unit::helper", "world::unit::helper"] {
            assert!(detached.signatures.contains_key(key), "{key}");
        }
        assert!(detached.is_trait_method_key("world::unit::Unit::step"));
        assert!(detached.is_trait_impl_method_key("world::unit::Unit::step"));
        assert_eq!(attached.trait_method_keys, detached.trait_method_keys);
        assert_eq!(attached.trait_impl_method_keys, detached.trait_impl_method_keys);
        // The detached layer no longer pins the shared global (no per-file copy of it stays alive).
        assert_eq!(std::sync::Arc::strong_count(&shared), 1);
    }
}
