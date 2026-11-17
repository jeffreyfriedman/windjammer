//! Signature lookups by identity (C3): a key the declarations name, never a search over homonyms.

use super::{FunctionSignature, SignatureRegistry};

impl SignatureRegistry {
    /// The signature a file registers for its own definition `name` (its analysis, kept by `keep_own_definitions`):
    /// this layer only, never a layer below it, where a same-named function of another module may sit.
    pub fn own_definition(&self, name: &str) -> Option<&FunctionSignature> {
        self.signatures.get(name)
    }

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
    /// A dependency's function (`crate_a::bytes::slice_bytes`) is registered from its `metadata.json` under the crate
    /// key (`crate_a::slice_bytes`).
    pub fn imported_free_function(&self, path: &str) -> Option<(String, &FunctionSignature)> {
        let segments: Vec<&str> = path.split("::").collect();
        let dependency_key = (segments.len() > 2).then(|| format!("{}::{}", segments[0], segments[segments.len() - 1]));
        [Some(path.to_string()), Some(format!("crate::{path}")), dependency_key].into_iter().flatten().find_map(|key| {
            let sig = self.layers().find_map(|reg| reg.signatures.get(&key))?;
            (!sig.has_self_receiver).then_some((key, sig))
        })
    }

    /// `member` of the type at module path `type_path` (`shapes::Mesh`, from the file's `use` or its own module): the
    /// key registered for that type's identity, with or without `crate::`. C3: never a search over homonym types.
    pub fn member_of_type_path(&self, type_path: &str, member: &str) -> Option<&FunctionSignature> {
        [format!("{type_path}::{member}"), format!("crate::{type_path}::{member}")]
            .into_iter()
            .find_map(|key| self.layers().find_map(|reg| reg.signatures.get(&key)))
    }

    /// True when the exact key `name` (on this layer or a layer below it) holds a method (`self` receiver):
    /// a trait or impl method registered under its bare name.
    pub fn key_holds_method(&self, name: &str) -> bool {
        self.layers().find_map(|reg| reg.signatures.get(name)).is_some_and(|sig| sig.has_self_receiver)
    }

    /// True when `key` (or a module-qualified alias) is a trait-impl method.
    pub fn is_trait_impl_method_key(&self, key: &str) -> bool {
        self.layers().any(|reg| reg.trait_impl_method_key_matches_local(key))
    }

    fn trait_impl_method_key_matches_local(&self, key: &str) -> bool {
        if self.trait_impl_method_keys.contains(key) {
            return true;
        }
        if let Some((parent, method)) = key.rsplit_once("::") {
            if parent.contains("::") {
                if let Some(type_name) = parent.rsplit("::").next() {
                    let short = format!("{type_name}::{method}");
                    if self.trait_impl_method_keys.contains(&short) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// `Type::method` keys from `impl Trait for Type` on this layer and every layer below it.
    pub fn all_trait_impl_method_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.layers().flat_map(|reg| reg.trait_impl_method_keys.iter().cloned()).collect();
        keys.sort();
        keys.dedup();
        keys
    }

    /// True when `key` (`Reader::check`) is a method of an inherent impl the program declares (`impl Reader`):
    /// a trait's contracts never apply to it.
    pub fn is_inherent_method_key(&self, key: &str) -> bool {
        self.layers().any(|reg| reg.inherent_method_keys.contains(key))
    }

    /// True when `sig` is `method` of an inherent impl (its receiver type `T` declares `impl T { fn method }`).
    pub fn is_inherent_method_of(&self, method: &str, sig: &FunctionSignature) -> bool {
        let receiver = match sig.param_types.first().filter(|_| sig.has_self_receiver) {
            Some(crate::parser::Type::Reference(inner) | crate::parser::Type::MutableReference(inner)) => inner.as_ref(),
            Some(other) => other,
            None => return false,
        };
        let crate::parser::Type::Custom(name) = receiver else { return false };
        let base = name.split('<').next().unwrap_or(name);
        self.is_inherent_method_key(&format!("{base}::{method}"))
    }

    /// Whether `key` can name the method a receiver of known type calls when `Type::method` itself is not registered:
    /// only a trait's method can (through a bound or a trait object). An inherent method or a free function of the
    /// same bare name (`Tabs::active` for a `RailItem` receiver) belongs to something else.
    pub fn key_may_name_method_of_known_receiver(&self, key: &str) -> bool {
        self.is_trait_method_key(key) || self.is_trait_impl_method_key(key)
    }

    /// [`Self::key_may_name_method_of_known_receiver`] for a receiver of type `receiver_type`, or any key when the
    /// receiver's type is unknown: the receiver type's own `Type::method` key, or a trait's method.
    pub fn key_may_name_method_of(&self, key: &str, receiver_type: Option<&str>, method: &str) -> bool {
        let Some(rt) = receiver_type else { return true };
        let base = rt.split('<').next().unwrap_or(rt);
        let own = format!("{base}::{method}");
        key == own || key.ends_with(&format!("::{own}")) || self.key_may_name_method_of_known_receiver(key)
    }
}

impl FunctionSignature {
    /// True when `self` and `other` can be views of one function (C3 identity): they declare the same formal
    /// parameter types and return type. Converged views of a function differ in ownership and emission flags,
    /// never in what the source declares; a same-named function of another module may differ in both.
    pub fn declares_same_function(&self, other: &FunctionSignature) -> bool {
        self.formal_param_types == other.formal_param_types && self.return_type == other.return_type
    }

    /// True when `self` can implement the trait method declared by `trait_sig` (E0053): the same receiver kind and
    /// the same number of formals. A same-named method of another trait with other formals is not its declaration.
    pub fn may_implement(&self, trait_sig: &FunctionSignature) -> bool {
        let arity = |s: &FunctionSignature| s.param_ownership.len().saturating_sub(usize::from(s.has_self_receiver));
        self.has_self_receiver == trait_sig.has_self_receiver && arity(self) == arity(trait_sig)
    }
}

/// Trait-key and dependency-type records (what the declarations of traits, impls and dependencies say).
impl SignatureRegistry {
    /// Record that `key` is a `Trait::method` definition (not an inherent impl).
    pub fn record_trait_method_key(&mut self, key: String) {
        self.trait_method_keys.insert(key);
    }

    /// Record that a dependency declares the type `name`, and whether its crate derives `Serialize` for it.
    pub fn record_dependency_type_serde(&mut self, name: String, derives_serialize: bool) {
        let entry = self.dependency_type_serde.entry(name).or_insert(derives_serialize);
        *entry = *entry && derives_serialize;
    }

    /// `Some(derives Serialize)` when a dependency declares the type `name` (this layer, then the layers below).
    pub fn dependency_type_serde(&self, name: &str) -> Option<bool> {
        self.layers().find_map(|reg| reg.dependency_type_serde.get(name).copied())
    }

    /// Record that `key` is a `Type::method` from `impl Trait for Type`, and the trait it declares it implements.
    pub fn record_trait_impl_method_key(&mut self, key: String, trait_name: Option<&str>) {
        if let Some(t) = trait_name {
            self.impl_method_traits.insert(key.clone(), t.split('<').next().unwrap_or(t).to_string());
        }
        self.trait_impl_method_keys.insert(key);
    }

    /// The trait an impl method `Type::method` implements, as its `impl Trait for Type` declares it.
    pub fn impl_method_trait(&self, impl_key: &str) -> Option<&str> {
        self.layers().find_map(|reg| reg.impl_method_traits.get(impl_key)).map(String::as_str)
    }

    /// The trait's own declaration of the method an impl method implements (`Trait::method`, by the trait the impl
    /// declares): `None` when the impl names no trait known to this registry.
    pub fn trait_definition_of_impl_method(&self, impl_key: &str) -> Option<(String, &FunctionSignature)> {
        let trait_path = self.impl_method_trait(impl_key)?;
        let member = impl_key.rsplit("::").next()?;
        let trait_leaf = trait_path.rsplit("::").next().unwrap_or(trait_path);
        [format!("{trait_path}::{member}"), format!("{trait_leaf}::{member}")]
            .into_iter()
            .find_map(|key| self.signature_declared_at(&key).map(|sig| (key, sig)))
    }
}
