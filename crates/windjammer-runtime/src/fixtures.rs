//! Fixture system for tests
//!
//! Provides a registry for test fixtures with automatic lifecycle management.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type FixtureFactory = Box<dyn Fn() -> Box<dyn Any + Send> + Send + Sync>;

/// Global fixture registry
static FIXTURE_REGISTRY: Mutex<Option<FixtureRegistry>> = Mutex::new(None);

/// Fixture registry for managing test resources
pub struct FixtureRegistry {
    factories: HashMap<String, FixtureFactory>,
    type_ids: HashMap<String, TypeId>,
}

impl FixtureRegistry {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
            type_ids: HashMap::new(),
        }
    }

    /// Register a fixture with a name and factory function
    pub fn register<T: 'static + Send>(
        &mut self,
        name: &str,
        factory: impl Fn() -> T + 'static + Send + Sync,
    ) {
        self.factories
            .insert(name.to_string(), Box::new(move || Box::new(factory())));
        self.type_ids.insert(name.to_string(), TypeId::of::<T>());
    }

    /// Get a fixture by name
    pub fn get<T: 'static>(&self, name: &str) -> Option<T> {
        let type_id = self.type_ids.get(name)?;
        if *type_id != TypeId::of::<T>() {
            return None;
        }

        let factory = self.factories.get(name)?;
        let boxed = factory();
        boxed.downcast::<T>().ok().map(|b| *b)
    }
}

impl Default for FixtureRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the global fixture registry
pub fn global_registry() -> Arc<Mutex<FixtureRegistry>> {
    let mut guard = FIXTURE_REGISTRY.lock().unwrap();
    if guard.is_none() {
        *guard = Some(FixtureRegistry::new());
    }
    drop(guard);

    // Return a reference to the registry
    Arc::new(Mutex::new(FixtureRegistry::new()))
}

/// Register a fixture globally
pub fn register_fixture<T: 'static + Send>(
    name: &str,
    factory: impl Fn() -> T + 'static + Send + Sync,
) {
    let registry = global_registry();
    let mut guard = registry.lock().unwrap();
    guard.register(name, factory);
}

/// Use a fixture by name
pub fn use_fixture<T: 'static>(name: &str) -> Option<T> {
    let registry = global_registry();
    let guard = registry.lock().unwrap();
    guard.get(name)
}

/// Fixture scope helper - automatically drops fixture at end of scope
pub struct FixtureScope<T> {
    value: Option<T>,
}

impl<T> FixtureScope<T> {
    pub fn new(value: T) -> Self {
        Self { value: Some(value) }
    }

    pub fn get(&self) -> &T {
        self.value.as_ref().unwrap()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.value.as_mut().unwrap()
    }
}

impl<T> Drop for FixtureScope<T> {
    fn drop(&mut self) {
        // Cleanup happens automatically via Drop
        self.value.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestData {
        value: i32,
    }

    #[test]
    fn test_fixture_registry() {
        let mut registry = FixtureRegistry::new();

        registry.register("test_data", || TestData { value: 42 });

        let data: Option<TestData> = registry.get("test_data");
        assert!(data.is_some());
        assert_eq!(data.unwrap().value, 42);
    }

    #[test]
    fn test_fixture_wrong_type() {
        let mut registry = FixtureRegistry::new();

        registry.register("test_data", || TestData { value: 42 });

        // Try to get with wrong type
        let data: Option<String> = registry.get("test_data");
        assert!(data.is_none());
    }

    #[test]
    fn test_fixture_not_found() {
        let registry = FixtureRegistry::new();

        let data: Option<TestData> = registry.get("nonexistent");
        assert!(data.is_none());
    }

    #[test]
    fn test_fixture_scope() {
        let scope = FixtureScope::new(TestData { value: 42 });
        assert_eq!(scope.get().value, 42);

        // scope is dropped here, cleanup happens automatically
    }

    #[test]
    fn test_fixture_scope_mut() {
        let mut scope = FixtureScope::new(TestData { value: 42 });
        scope.get_mut().value = 100;
        assert_eq!(scope.get().value, 100);
    }
}
