//! Fine-grained reactive signals for Windjammer UI
//! 
//! This module provides the core reactive primitives for the UI framework:
//! - `Signal<T>`: Reactive value that notifies subscribers on change
//! - `Computed<T>`: Derived value that auto-updates when dependencies change
//! - `Effect`: Side effect that runs when dependencies change
//! 
//! Inspired by Solid.js, Vue 3, and Leptos.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Unique identifier for signals
pub type SignalId = usize;

/// Unique identifier for effects
pub type EffectId = usize;

/// Global reactive context (thread-local for single-threaded WASM)
thread_local! {
    static REACTIVE_CONTEXT: RefCell<ReactiveContext> = RefCell::new(ReactiveContext::new());
    static EFFECT_REGISTRY: RefCell<HashMap<EffectId, EffectHandle>> = RefCell::new(HashMap::new());
}

/// Reactive context tracks the current effect being executed
struct ReactiveContext {
    /// The effect currently being executed (for dependency tracking)
    current_effect: Option<EffectId>,
    /// Cleanup functions to run when effects are disposed
    cleanups: HashMap<EffectId, Vec<Box<dyn Fn()>>>,
}

impl ReactiveContext {
    fn new() -> Self {
        Self {
            current_effect: None,
            cleanups: HashMap::new(),
        }
    }
}

/// Handle to an effect for execution
struct EffectHandle {
    f: Rc<dyn Fn()>,
}

/// Core reactive primitive - a value that notifies subscribers when it changes
pub struct Signal<T> {
    id: SignalId,
    value: Rc<RefCell<T>>,
    subscribers: Rc<RefCell<HashSet<EffectId>>>,
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            value: Rc::clone(&self.value),
            subscribers: Rc::clone(&self.subscribers),
        }
    }
}

impl<T: Clone> Signal<T> {
    /// Create a new signal with an initial value
    pub fn new(value: T) -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            value: Rc::new(RefCell::new(value)),
            subscribers: Rc::new(RefCell::new(HashSet::new())),
        }
    }

    /// Get the current value (tracks dependency in reactive context)
    pub fn get(&self) -> T {
        // Track this read in the current reactive context
        REACTIVE_CONTEXT.with(|ctx| {
            let ctx = ctx.borrow();
            if let Some(effect_id) = ctx.current_effect {
                self.subscribers.borrow_mut().insert(effect_id);
            }
        });

        self.value.borrow().clone()
    }

    /// Get the current value without tracking (for debugging)
    pub fn get_untracked(&self) -> T {
        self.value.borrow().clone()
    }

    /// Set a new value and notify subscribers
    pub fn set(&self, value: T) {
        *self.value.borrow_mut() = value;
        self.notify();
    }

    /// Update the value using a function and notify subscribers
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        f(&mut self.value.borrow_mut());
        self.notify();
    }

    /// Notify all subscribers that the value has changed
    fn notify(&self) {
        let subscribers = self.subscribers.borrow().clone();
        for effect_id in subscribers {
            EFFECT_REGISTRY.with(|registry| {
                if let Some(effect) = registry.borrow().get(&effect_id) {
                    (effect.f)();
                }
            });
        }
    }

    /// Get the signal's unique ID
    pub fn id(&self) -> SignalId {
        self.id
    }
}

impl<T: Clone + std::fmt::Debug> std::fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signal")
            .field("id", &self.id)
            .field("value", &self.get_untracked())
            .finish()
    }
}

/// Computed signal - a derived value that automatically updates when dependencies change
pub struct Computed<T> {
    signal: Signal<T>,
    _effect_id: EffectId,
}

impl<T> Clone for Computed<T> {
    fn clone(&self) -> Self {
        Self {
            signal: self.signal.clone(),
            _effect_id: self._effect_id,
        }
    }
}

impl<T: Clone + 'static> Computed<T> {
    /// Create a new computed value
    pub fn new<F>(compute: F) -> Self
    where
        F: Fn() -> T + 'static,
    {
        let compute = Rc::new(compute);

        // Compute initial value
        let initial_value = compute();
        let signal = Signal::new(initial_value);

        // Create effect that updates signal when dependencies change
        let signal_clone = signal.clone();
        let compute_clone = compute.clone();
        let effect_id = Effect::new(move || {
            let new_value = compute_clone();
            // Use set_untracked to avoid infinite loops
            *signal_clone.value.borrow_mut() = new_value;
        });

        Self {
            signal,
            _effect_id: effect_id,
        }
    }

    /// Get the current computed value (tracks dependency)
    pub fn get(&self) -> T {
        self.signal.get()
    }

    /// Get the current computed value without tracking
    pub fn get_untracked(&self) -> T {
        self.signal.get_untracked()
    }
}

impl<T: Clone + std::fmt::Debug> std::fmt::Debug for Computed<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Computed")
            .field("value", &self.get_untracked())
            .finish()
    }
}

/// Effect - a side effect that runs when its dependencies change
pub struct Effect {
    id: EffectId,
}

impl Effect {
    /// Create a new effect
    pub fn new<F>(f: F) -> EffectId
    where
        F: Fn() + 'static,
    {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let f = Rc::new(f);

        // Register effect
        EFFECT_REGISTRY.with(|registry| {
            registry.borrow_mut().insert(id, EffectHandle { f: f.clone() });
        });

        // Run effect once to establish dependencies
        Self::run_effect(id, &f);

        id
    }

    /// Run an effect with the given ID as the current reactive context
    fn run_effect(id: EffectId, f: &Rc<dyn Fn()>) {
        REACTIVE_CONTEXT.with(|ctx| {
            // Save previous effect
            let prev_effect = ctx.borrow().current_effect;

            // Set current effect
            ctx.borrow_mut().current_effect = Some(id);

            // Run the effect
            f();

            // Restore previous effect
            ctx.borrow_mut().current_effect = prev_effect;
        });
    }

    /// Dispose of an effect (stop tracking and run cleanup)
    pub fn dispose(id: EffectId) {
        // Run cleanup functions
        REACTIVE_CONTEXT.with(|ctx| {
            if let Some(cleanups) = ctx.borrow_mut().cleanups.remove(&id) {
                for cleanup in cleanups {
                    cleanup();
                }
            }
        });

        // Remove from registry
        EFFECT_REGISTRY.with(|registry| {
            registry.borrow_mut().remove(&id);
        });
    }
}

/// Register a cleanup function to run when the current effect is disposed
pub fn on_cleanup<F>(cleanup: F)
where
    F: Fn() + 'static,
{
    REACTIVE_CONTEXT.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        if let Some(effect_id) = ctx.current_effect {
            ctx.cleanups
                .entry(effect_id)
                .or_insert_with(Vec::new)
                .push(Box::new(cleanup));
        }
    });
}

/// Create a reactive scope that tracks dependencies
pub fn create_effect<F>(f: F) -> EffectId
where
    F: Fn() + 'static,
{
    Effect::new(f)
}

/// Create a computed value
pub fn create_computed<T, F>(compute: F) -> Computed<T>
where
    T: Clone + 'static,
    F: Fn() -> T + 'static,
{
    Computed::new(compute)
}

/// Create a signal
pub fn create_signal<T: Clone>(value: T) -> Signal<T> {
    Signal::new(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_basic() {
        let count = Signal::new(0);
        assert_eq!(count.get(), 0);

        count.set(5);
        assert_eq!(count.get(), 5);

        count.update(|c| *c += 10);
        assert_eq!(count.get(), 15);
    }

    #[test]
    fn test_signal_effect() {
        let count = Signal::new(0);
        let result = Rc::new(RefCell::new(0));

        let result_clone = result.clone();
        let count_clone = count.clone();
        Effect::new(move || {
            *result_clone.borrow_mut() = count_clone.get() * 2;
        });

        assert_eq!(*result.borrow(), 0);

        count.set(5);
        assert_eq!(*result.borrow(), 10);

        count.set(10);
        assert_eq!(*result.borrow(), 20);
    }

    #[test]
    fn test_computed() {
        let count = Signal::new(5);
        let count_clone = count.clone();
        let doubled = Computed::new(move || count_clone.get() * 2);

        assert_eq!(doubled.get(), 10);

        count.set(10);
        assert_eq!(doubled.get(), 20);

        count.set(7);
        assert_eq!(doubled.get(), 14);
    }

    #[test]
    fn test_multiple_dependencies() {
        let a = Signal::new(2);
        let b = Signal::new(3);

        let a_clone = a.clone();
        let b_clone = b.clone();
        let sum = Computed::new(move || a_clone.get() + b_clone.get());

        assert_eq!(sum.get(), 5);

        a.set(10);
        assert_eq!(sum.get(), 13);

        b.set(7);
        assert_eq!(sum.get(), 17);
    }

    #[test]
    fn test_effect_runs_immediately() {
        let ran = Rc::new(RefCell::new(false));
        let ran_clone = ran.clone();

        Effect::new(move || {
            *ran_clone.borrow_mut() = true;
        });

        assert!(*ran.borrow());
    }

    #[test]
    fn test_untracked_read() {
        let count = Signal::new(0);
        let effect_count = Rc::new(RefCell::new(0));

        let effect_count_clone = effect_count.clone();
        let count_clone = count.clone();
        Effect::new(move || {
            // This should not trigger on untracked changes
            let _ = count_clone.get_untracked();
            *effect_count_clone.borrow_mut() += 1;
        });

        assert_eq!(*effect_count.borrow(), 1); // Initial run

        count.set(5);
        assert_eq!(*effect_count.borrow(), 1); // Should not re-run
    }
}


