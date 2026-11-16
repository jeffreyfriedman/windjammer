//! Tables other passes derive from the stdlib baseline (`type_inference`'s signature tables, the semantic check's
//! `std/rust_core.wji` surface). They are built once per process, on the first request, and kept with the baseline
//! registry they come from, so no pass needs a static of its own.

use super::SignatureRegistry;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Mutex;

/// Values keyed by their type, each built on first use and kept for the life of the process.
#[derive(Default)]
pub struct DerivedTables(Mutex<HashMap<TypeId, &'static (dyn Any + Send + Sync)>>);

impl Clone for DerivedTables {
    /// A copy of a registry starts with no derived tables (they belong to the stdlib baseline).
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl std::fmt::Debug for DerivedTables {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DerivedTables")
    }
}

impl DerivedTables {
    /// The `T` of this registry, built by `build` the first time anyone asks (`T` names the table: use a newtype).
    pub fn get_or_build<T: Any + Send + Sync>(&self, build: impl FnOnce() -> T) -> &'static T {
        let mut tables = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(found) = tables.get(&TypeId::of::<T>()) {
            return found.downcast_ref::<T>().expect("keyed by its type");
        }
        let leaked: &'static T = Box::leak(Box::new(build()));
        tables.insert(TypeId::of::<T>(), leaked);
        leaked
    }
}

impl SignatureRegistry {
    /// The tables derived from this registry (see [`DerivedTables`]).
    pub fn derived(&self) -> &DerivedTables {
        &self.derived
    }
}
