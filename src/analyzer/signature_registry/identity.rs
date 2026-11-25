//! Signature lookups by identity (C3): a key the declarations name, never a search over homonyms.

use super::{FunctionSignature, SignatureRegistry};
use std::collections::HashMap;

/// Every declaration of a key that two impls of one type share with different formals (`impl Mul<f32> for V` and
/// `impl Mul<V> for V` both define `V::mul`): `signatures` holds the last, this holds all, so a call is resolved by
/// its arguments' types. Not part of the metadata snapshot: a dependency's overloads fall back to the last one.
#[derive(Clone, Debug, Default)]
pub(super) struct Overloads(HashMap<String, Vec<FunctionSignature>>);

impl Overloads {
    /// `sig` collides with the already registered `existing` under `name`: both are overloads (methods only).
    pub(super) fn record_collision(&mut self, name: &str, existing: &FunctionSignature, sig: &FunctionSignature) {
        if existing.has_self_receiver && sig.has_self_receiver {
            let all = self.0.entry(name.to_string()).or_default();
            if all.is_empty() {
                all.push(existing.clone());
            }
        }
    }

    /// `sig` is registered under `name`: a refresh of one overload replaces it, a new one is added.
    pub(super) fn note(&mut self, name: &str, sig: &FunctionSignature) {
        if let Some(all) = self.0.get_mut(name) {
            match all.iter_mut().find(|o| o.declares_same_function(sig)) {
                Some(slot) => *slot = sig.clone(),
                None => all.push(sig.clone()),
            }
        }
    }

    pub(super) fn extend_from(&mut self, other: &Overloads) {
        self.0.extend(other.0.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
}

impl SignatureRegistry {
    /// Every declaration of method key `key` when several impls of one type define it with different formals
    /// (this layer, then the layers below); empty when the key has one declaration.
    pub fn overloads_at(&self, key: &str) -> &[FunctionSignature] {
        self.layers().find_map(|reg| reg.overloads.0.get(key)).map_or(&[], Vec::as_slice)
    }

    /// Index `Type::method` under `method`; buckets stay sorted so "first homonym" is deterministic.
    /// True when [`Self::add_function`]`(name, sig)` would leave this registry unchanged: this layer already
    /// holds exactly `sig` under `name` and `name` is in the method index.
    pub fn add_function_is_noop(&self, name: &str, sig: &FunctionSignature) -> bool {
        if self.signatures.get(name) != Some(sig) {
            return false;
        }
        let Some((_, suffix)) = name.rsplit_once("::") else { return true };
        self.method_index
            .get(suffix)
            .is_some_and(|bucket| bucket.binary_search_by(|k| k.as_str().cmp(name)).is_ok())
    }

    pub(super) fn index_method_key(&mut self, name: &str) {
        let Some((_, suffix)) = name.rsplit_once("::") else { return };
        let bucket = self.method_index.entry(suffix.to_string()).or_default();
        if let Err(pos) = bucket.binary_search_by(|k| k.as_str().cmp(name)) { bucket.insert(pos, name.to_string()); }
    }

    /// The one `{Type}::{suffix}` a bare method name (no receiver type) can name, or `None` when it is ambiguous: the
    /// nearest layer that registers the name must hold exactly one key, and no layer may hold the method on a builtin
    /// value type (`Vec`, `String`, maps, `Option`...). A receiver whose type is not inferred yet may be one of those
    /// (`let mut seen = Vec::new()`), so a user `Column::contains` or `CallStack::push` never answers for
    /// `Vec::contains` / `Vec::push`, wherever the std homonym sits. Runtime module types (`Timer::reset`) do not veto.
    pub fn find_unique_signature_ending_with(&self, suffix: &str) -> Option<&FunctionSignature> {
        let nearest = self.layers().find_map(|reg| reg.method_index.get(suffix).filter(|k| !k.is_empty()).map(|k| (reg, k)));
        let (reg, keys) = nearest?;
        let [only] = keys.as_slice() else { return None };
        let builtin_homonym = self.signatures_for_method_name(suffix).any(|(key, _)| key != only && key_is_builtin_type_method(key));
        if builtin_homonym {
            return None;
        }
        reg.signatures.get(only)
    }

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

    /// False when `self` and `other` are two functions that share a name: a static fn and a method (`Effect::set_variable`
    /// and `Realm::set_variable`; their parameter slots do not line up) or the methods of two types (`Column::contains`,
    /// `Vec::contains`). A refresh of one never updates the other. Unqualified names and module paths stay comparable.
    pub fn may_be_view_of(&self, other: &FunctionSignature) -> bool {
        let owner = |s: &FunctionSignature| {
            let (path, _) = s.name.rsplit_once("::")?;
            let leaf = path.rsplit("::").next().unwrap_or(path);
            let leaf = leaf.split('<').next().unwrap_or(leaf);
            leaf.starts_with(|c: char| c.is_ascii_uppercase()).then(|| leaf.to_string())
        };
        let distinct_owners = matches!((owner(self), owner(other)), (Some(a), Some(b)) if a != b && a != "Self" && b != "Self");
        self.has_self_receiver == other.has_self_receiver && !distinct_owners
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

/// `Vec::contains`, `String::push`, `HashMap::get`: a method of a builtin value type (not of a runtime module's type).
fn key_is_builtin_type_method(key: &str) -> bool {
    let owner = key.rsplit_once("::").map_or("", |(o, _)| o.rsplit("::").next().unwrap_or(o));
    crate::type_classification::is_stdlib_container(owner) || matches!(owner, "String" | "str" | "string")
}
