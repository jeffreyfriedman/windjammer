//! Optimized reactivity system with performance improvements
//!
//! Key optimizations:
//! 1. SmallVec for subscribers (avoid heap allocation for <= 4 subscribers)
//! 2. Batched updates to avoid redundant notifications
//! 3. Copy optimization for cheap-to-copy types
//! 4. Effect deduplication

use smallvec::SmallVec;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub type SignalId = usize;
pub type EffectId = usize;

thread_local! {
    static REACTIVE_CONTEXT: RefCell<ReactiveContext> = RefCell::new(ReactiveContext::default());
    static EFFECT_REGISTRY: RefCell<HashMap<EffectId, Effect>> = RefCell::new(HashMap::new());
    static UPDATE_BATCH: RefCell<UpdateBatch> = RefCell::new(UpdateBatch::default());
}

#[derive(Default)]
struct ReactiveContext {
    current_effect: Option<EffectId>,
}

struct Effect {
    f: Box<dyn Fn()>,
}

/// Batch multiple updates together to avoid redundant notifications
#[derive(Default)]
struct UpdateBatch {
    pending_effects: HashSet<EffectId>,
    is_batching: bool,
}

impl UpdateBatch {
    fn start_batch() {
        UPDATE_BATCH.with(|batch| {
            batch.borrow_mut().is_batching = true;
        });
    }

    fn end_batch() {
        UPDATE_BATCH.with(|batch| {
            let mut batch = batch.borrow_mut();
            batch.is_batching = false;

            // Execute all pending effects
            let pending: Vec<_> = batch.pending_effects.drain().collect();
            drop(batch); // Release borrow before executing effects

            for effect_id in pending {
                EFFECT_REGISTRY.with(|registry| {
                    if let Some(effect) = registry.borrow().get(&effect_id) {
                        (effect.f)();
                    }
                });
            }
        });
    }

    fn add_effect(effect_id: EffectId) {
        UPDATE_BATCH.with(|batch| {
            let mut batch = batch.borrow_mut();
            if batch.is_batching {
                batch.pending_effects.insert(effect_id);
            } else {
                // Execute immediately if not batching
                drop(batch); // Release borrow
                EFFECT_REGISTRY.with(|registry| {
                    if let Some(effect) = registry.borrow().get(&effect_id) {
                        (effect.f)();
                    }
                });
            }
        });
    }
}

/// Optimized Signal using SmallVec for subscribers
#[derive(Clone)]
pub struct Signal<T: Clone> {
    id: SignalId,
    value: Rc<RefCell<T>>,
    // Use SmallVec to avoid heap allocation for <= 4 subscribers
    subscribers: Rc<RefCell<SmallVec<[EffectId; 4]>>>,
}

impl<T: Clone> Signal<T> {
    pub fn new(value: T) -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            value: Rc::new(RefCell::new(value)),
            subscribers: Rc::new(RefCell::new(SmallVec::new())),
        }
    }

    pub fn get(&self) -> T {
        // Track this read in the current reactive context
        REACTIVE_CONTEXT.with(|ctx| {
            let ctx = ctx.borrow();
            if let Some(effect_id) = ctx.current_effect {
                let mut subs = self.subscribers.borrow_mut();
                if !subs.contains(&effect_id) {
                    subs.push(effect_id);
                }
            }
        });

        self.value.borrow().clone()
    }

    pub fn get_untracked(&self) -> T {
        self.value.borrow().clone()
    }

    pub fn set(&self, value: T) {
        *self.value.borrow_mut() = value;
        self.notify();
    }

    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        f(&mut self.value.borrow_mut());
        self.notify();
    }

    fn notify(&self) {
        let subscribers = self.subscribers.borrow().clone();
        for effect_id in subscribers.iter() {
            UpdateBatch::add_effect(*effect_id);
        }
    }

    pub fn id(&self) -> SignalId {
        self.id
    }
}

/// Optimized Signal for Copy types (no cloning needed)
#[derive(Clone)]
pub struct CopySignal<T: Copy> {
    id: SignalId,
    value: Rc<RefCell<T>>,
    subscribers: Rc<RefCell<SmallVec<[EffectId; 4]>>>,
}

impl<T: Copy> CopySignal<T> {
    pub fn new(value: T) -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            value: Rc::new(RefCell::new(value)),
            subscribers: Rc::new(RefCell::new(SmallVec::new())),
        }
    }

    pub fn get(&self) -> T {
        REACTIVE_CONTEXT.with(|ctx| {
            let ctx = ctx.borrow();
            if let Some(effect_id) = ctx.current_effect {
                let mut subs = self.subscribers.borrow_mut();
                if !subs.contains(&effect_id) {
                    subs.push(effect_id);
                }
            }
        });

        *self.value.borrow()
    }

    pub fn get_untracked(&self) -> T {
        *self.value.borrow()
    }

    pub fn set(&self, value: T) {
        *self.value.borrow_mut() = value;
        self.notify();
    }

    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        f(&mut self.value.borrow_mut());
        self.notify();
    }

    fn notify(&self) {
        let subscribers = self.subscribers.borrow().clone();
        for effect_id in subscribers.iter() {
            UpdateBatch::add_effect(*effect_id);
        }
    }

    pub fn id(&self) -> SignalId {
        self.id
    }
}

/// Create an effect that runs when its dependencies change
pub fn create_effect<F>(f: F) -> EffectId
where
    F: Fn() + 'static,
{
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

    let effect = Effect { f: Box::new(f) };
    EFFECT_REGISTRY.with(|registry| {
        registry.borrow_mut().insert(id, effect);
    });

    // Run the effect once to establish dependencies
    REACTIVE_CONTEXT.with(|ctx| {
        ctx.borrow_mut().current_effect = Some(id);
    });

    EFFECT_REGISTRY.with(|registry| {
        if let Some(effect) = registry.borrow().get(&id) {
            (effect.f)();
        }
    });

    REACTIVE_CONTEXT.with(|ctx| {
        ctx.borrow_mut().current_effect = None;
    });

    id
}

/// Batch multiple updates together
pub fn batch<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    UpdateBatch::start_batch();
    let result = f();
    UpdateBatch::end_batch();
    result
}

/// Dispose an effect
pub fn dispose_effect(id: EffectId) {
    EFFECT_REGISTRY.with(|registry| {
        registry.borrow_mut().remove(&id);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_basic() {
        let signal = Signal::new(42);
        assert_eq!(signal.get(), 42);

        signal.set(100);
        assert_eq!(signal.get(), 100);
    }

    #[test]
    fn test_copy_signal() {
        let signal = CopySignal::new(42);
        assert_eq!(signal.get(), 42);

        signal.set(100);
        assert_eq!(signal.get(), 100);
    }

    #[test]
    fn test_effect() {
        let signal = Signal::new(0);
        let result = Rc::new(RefCell::new(0));
        let result_clone = result.clone();
        let signal_clone = signal.clone();

        let _effect_id = create_effect(move || {
            *result_clone.borrow_mut() = signal_clone.get();
        });

        assert_eq!(*result.borrow(), 0);

        signal.set(42);
        assert_eq!(*result.borrow(), 42);
    }

    #[test]
    fn test_batched_updates() {
        let signal1 = Signal::new(0);
        let signal2 = Signal::new(0);
        let count = Rc::new(RefCell::new(0));
        let count_clone = count.clone();
        let signal1_clone = signal1.clone();
        let signal2_clone = signal2.clone();

        let _effect_id = create_effect(move || {
            let _ = signal1_clone.get();
            let _ = signal2_clone.get();
            *count_clone.borrow_mut() += 1;
        });

        // Reset count after initial effect run
        *count.borrow_mut() = 0;

        // Without batching, this would trigger 2 effect runs
        batch(|| {
            signal1.set(1);
            signal2.set(2);
        });

        // With batching, effect should only run once
        assert_eq!(*count.borrow(), 1);
    }

    #[test]
    fn test_smallvec_optimization() {
        let signal = Signal::new(0);
        let s1 = signal.clone();
        let s2 = signal.clone();
        let s3 = signal.clone();

        // Add 3 subscribers (should fit in SmallVec without heap allocation)
        let _e1 = create_effect(move || {
            let _ = s1.get();
        });
        let _e2 = create_effect(move || {
            let _ = s2.get();
        });
        let _e3 = create_effect(move || {
            let _ = s3.get();
        });

        // Verify subscribers are tracked
        assert_eq!(signal.subscribers.borrow().len(), 3);
    }
}
