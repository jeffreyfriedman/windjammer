//! The scanned runtime layer of the stdlib baseline, persisted between processes.
//!
//! `stdlib_scanner::populate_runtime_signatures` is a pure function of the compiler build (the scanner code and
//! the `windjammer-runtime` / `std` trees baked into the binary), and it costs a third of a `wj check`'s
//! instructions. When the driver installs a [`LayerStore`], the populated layer is stored under a key made of the
//! build key and the fingerprint of those baked-in trees, and a later process loads it instead of scanning.
//! Serialization is sorted (B-trees), so one build always writes the same text. Anything unexpected (no store,
//! unknown build, unreadable or unparsable entry) is a miss: the scan runs, as before.

use super::{FunctionSignature, SignatureRegistry};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::{Arc, Mutex};

const FORMAT: u32 = 1;

/// Where entries live: opaque text (or bytes) under a 64-digit hex key. `put` must be atomic for readers.
pub trait LayerStore: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    fn put(&self, key: &str, text: &str);
    /// The entry as bytes (a binary table); a store without byte entries has none.
    fn get_bytes(&self, _key: &str) -> Option<Vec<u8>> {
        None
    }
    fn put_bytes(&self, _key: &str, _bytes: &[u8]) {}
}

/// The store and the key of the compiler build it serves.
static STORE: Mutex<Option<(Arc<dyn LayerStore>, String)>> = Mutex::new(None);

/// Share the scanned runtime layer through `store` (the driver calls this before the first registry is built).
/// `compiler_key` is the identity of this compiler build by content (`check::shared_cache::compiler_key`; a build
/// of unknown origin has none and installs nothing): the scanner code is part of the build.
pub fn install_layer_store(store: Arc<dyn LayerStore>, compiler_key: String) {
    *STORE.lock().unwrap_or_else(|e| e.into_inner()) = Some((store, compiler_key));
}

/// The store and the key of a table that is a function of the build and the trees baked into it (`kind` names
/// the table: `runtime-layer`, `stdlib-tables`).
pub fn shared_entry(kind: &str) -> Option<(Arc<dyn LayerStore>, String)> {
    let (store, compiler) = STORE.lock().unwrap_or_else(|e| e.into_inner()).clone()?;
    let text = format!("wj-{kind}\n{FORMAT}\n{compiler}\n{}", crate::embedded_assets::scan_inputs_fingerprint());
    Some((store, Sha256::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()))
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Snapshot {
    signatures: BTreeMap<String, FunctionSignature>,
    type_collision_keys: BTreeSet<String>,
    ownership_collision_keys: BTreeSet<String>,
    method_index: BTreeMap<String, Vec<String>>,
    trait_method_keys: BTreeSet<String>,
    trait_impl_method_keys: BTreeSet<String>,
    runtime_std_modules: BTreeSet<String>,
    runtime_std_rust_stems: BTreeMap<String, String>,
    runtime_type_modules: BTreeMap<String, String>,
    runtime_exported_types: BTreeMap<String, Vec<String>>,
    runtime_type_fields: BTreeMap<String, Vec<(String, String)>>,
    runtime_non_copy_types: BTreeSet<String>,
    runtime_non_clone_types: BTreeSet<String>,
    taint_sanitizer_callees: BTreeSet<String>,
}

/// The stored runtime layer read back: `Snapshot`'s fields in its order, each in the collection type the registry
/// keeps (a set or a map is a length and its items in bincode, whatever collection wrote it), so loading builds the
/// registry's own collections directly. Written sorted (from `Snapshot`), so one build always stores the same bytes.
#[derive(serde::Deserialize)]
struct Stored {
    signatures: BTreeMap<String, FunctionSignature>,
    type_collision_keys: HashSet<String>,
    ownership_collision_keys: HashSet<String>,
    method_index: HashMap<String, Vec<String>>,
    trait_method_keys: Vec<String>,
    trait_impl_method_keys: HashSet<String>,
    runtime_std_modules: HashSet<String>,
    runtime_std_rust_stems: HashMap<String, String>,
    runtime_type_modules: HashMap<String, String>,
    runtime_exported_types: HashMap<String, Vec<String>>,
    runtime_type_fields: HashMap<String, Vec<(String, String)>>,
    runtime_non_copy_types: HashSet<String>,
    runtime_non_clone_types: HashSet<String>,
    taint_sanitizer_callees: HashSet<String>,
}

impl Stored {
    fn into_registry(self) -> SignatureRegistry {
        let mut r = SignatureRegistry::empty();
        r.signatures = self.signatures;
        r.type_collision_keys = self.type_collision_keys;
        r.ownership_collision_keys = self.ownership_collision_keys;
        r.method_index = self.method_index;
        r.trait_method_keys = self.trait_method_keys.into_iter().collect();
        r.trait_impl_method_keys = self.trait_impl_method_keys;
        r.runtime_std_modules = self.runtime_std_modules;
        r.runtime_std_rust_stems = self.runtime_std_rust_stems;
        r.runtime_type_modules = self.runtime_type_modules;
        r.runtime_exported_types = self.runtime_exported_types;
        r.runtime_type_fields = self.runtime_type_fields;
        r.runtime_non_copy_types = self.runtime_non_copy_types;
        r.runtime_non_clone_types = self.runtime_non_clone_types;
        r.taint_sanitizer_callees = self.taint_sanitizer_callees;
        r
    }
}

impl Snapshot {
    fn of(r: &SignatureRegistry) -> Snapshot {
        Snapshot {
            signatures: r.signatures.clone(),
            type_collision_keys: r.type_collision_keys.iter().cloned().collect(),
            ownership_collision_keys: r.ownership_collision_keys.iter().cloned().collect(),
            method_index: r.method_index.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            trait_method_keys: r.trait_method_keys.iter().cloned().collect(),
            trait_impl_method_keys: r.trait_impl_method_keys.iter().cloned().collect(),
            runtime_std_modules: r.runtime_std_modules.iter().cloned().collect(),
            runtime_std_rust_stems: r.runtime_std_rust_stems.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            runtime_type_modules: r.runtime_type_modules.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            runtime_exported_types: r.runtime_exported_types.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            runtime_type_fields: r.runtime_type_fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            runtime_non_copy_types: r.runtime_non_copy_types.iter().cloned().collect(),
            runtime_non_clone_types: r.runtime_non_clone_types.iter().cloned().collect(),
            taint_sanitizer_callees: r.taint_sanitizer_callees.iter().cloned().collect(),
        }
    }

    fn into_registry(self) -> SignatureRegistry {
        let mut r = SignatureRegistry::empty();
        r.signatures = self.signatures;
        r.type_collision_keys = self.type_collision_keys.into_iter().collect();
        r.ownership_collision_keys = self.ownership_collision_keys.into_iter().collect();
        r.method_index = self.method_index.into_iter().collect();
        r.trait_method_keys = self.trait_method_keys.into_iter().collect();
        r.trait_impl_method_keys = self.trait_impl_method_keys.into_iter().collect();
        r.runtime_std_modules = self.runtime_std_modules.into_iter().collect();
        r.runtime_std_rust_stems = self.runtime_std_rust_stems.into_iter().collect();
        r.runtime_type_modules = self.runtime_type_modules.into_iter().collect();
        r.runtime_exported_types = self.runtime_exported_types.into_iter().collect();
        r.runtime_type_fields = self.runtime_type_fields.into_iter().collect();
        r.runtime_non_copy_types = self.runtime_non_copy_types.into_iter().collect();
        r.runtime_non_clone_types = self.runtime_non_clone_types.into_iter().collect();
        r.taint_sanitizer_callees = self.taint_sanitizer_callees.into_iter().collect();
        r
    }
}

/// The populated runtime layer: loaded from the shared store when it has this build's entry, else scanned (and stored).
/// Stored as bincode (`runtime-layer-bin`): loading is a length-prefixed read into the registry's collections, no
/// text to parse (the JSON entry cost 13M instructions, the most expensive phase of a small program's start).
pub(super) fn scanned_runtime_layer() -> SignatureRegistry {
    let shared = shared_entry("runtime-layer-bin");
    if let Some((store, key)) = &shared {
        if let Some(stored) = store.get_bytes(key).and_then(|bytes| bincode::deserialize::<Stored>(&bytes).ok()) {
            return stored.into_registry();
        }
    }
    let mut runtime = SignatureRegistry::empty();
    if let Err(e) = crate::stdlib_scanner::populate_runtime_signatures(&mut runtime) {
        eprintln!("Warning: Failed to scan runtime signatures: {}", e);
        eprintln!("Continuing with empty registry - may generate incorrect borrows");
        return runtime;
    }
    if let Some((store, key)) = &shared {
        if let Ok(bytes) = bincode::serialize(&Snapshot::of(&runtime)) {
            store.put_bytes(key, &bytes);
        }
    }
    runtime
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The stored layer is the scanned layer: a round trip through the text loses nothing, and the text is stable.
    #[test]
    fn a_stored_layer_round_trips_to_the_scanned_layer() {
        let mut scanned = SignatureRegistry::empty();
        crate::stdlib_scanner::populate_runtime_signatures(&mut scanned).expect("scan");
        let text = serde_json::to_string(&Snapshot::of(&scanned)).unwrap();
        let back = serde_json::from_str::<Snapshot>(&text).unwrap().into_registry();
        assert_eq!(serde_json::to_string(&Snapshot::of(&back)).unwrap(), text);
        assert!(scanned.signatures.len() > 500, "the scan found the runtime signatures");
        assert_eq!(format!("{:?}", back.signatures), format!("{:?}", scanned.signatures));
        let sorted = |s: &HashSetOfStrings| -> Vec<String> { let mut v: Vec<String> = s.iter().cloned().collect(); v.sort(); v };
        assert_eq!(sorted(&back.runtime_non_copy_types), sorted(&scanned.runtime_non_copy_types));
    }

    type HashSetOfStrings = std::collections::HashSet<String>;

    /// The binary entry a process loads is the scanned layer: written from `Snapshot`, read as `Stored`, every
    /// table equal (and the signatures in the same order), and one scan always writes the same bytes.
    #[test]
    fn the_binary_layer_reads_back_as_the_scanned_layer() {
        let mut scanned = SignatureRegistry::empty();
        crate::stdlib_scanner::populate_runtime_signatures(&mut scanned).expect("scan");
        let bytes = bincode::serialize(&Snapshot::of(&scanned)).unwrap();
        let back = bincode::deserialize::<Stored>(&bytes).unwrap().into_registry();
        assert_eq!(bincode::serialize(&Snapshot::of(&back)).unwrap(), bytes, "a round trip writes the same bytes");
        assert_eq!(format!("{:?}", back.signatures), format!("{:?}", scanned.signatures));
        let sorted = |s: &HashSetOfStrings| -> Vec<String> { let mut v: Vec<String> = s.iter().cloned().collect(); v.sort(); v };
        assert_eq!(sorted(&back.runtime_non_copy_types), sorted(&scanned.runtime_non_copy_types));
        assert_eq!(back.trait_method_keys, scanned.trait_method_keys);
        assert_eq!(back.runtime_type_fields.len(), scanned.runtime_type_fields.len());
        let mut again = SignatureRegistry::empty();
        crate::stdlib_scanner::populate_runtime_signatures(&mut again).expect("scan");
        assert_eq!(bincode::serialize(&Snapshot::of(&again)).unwrap(), bytes, "one build stores the same bytes");
    }
}
