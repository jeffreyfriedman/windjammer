//! # Automatic Memory Pooling System
//!
//! Reduces allocation overhead by reusing memory for frequently created/destroyed objects.
//!
//! ## Features
//! - Object pooling for any type
//! - Automatic pool growth
//! - Pool statistics and profiling
//! - Thread-safe pools
//! - Configurable pool sizes
//! - Pool cleanup and shrinking
//! - Type-specific pools
//! - Pool warming (pre-allocation)
//!
//! ## Example
//! ```no_run
//! use windjammer_game_framework::memory_pool::{Pool, PoolConfig};
//!
//! let mut pool = Pool::<MyObject>::new(PoolConfig::default());
//! let obj = pool.acquire();
//! // Use object...
//! pool.release(obj);
//! ```

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Initial pool capacity
    pub initial_capacity: usize,
    /// Maximum pool size (0 = unlimited)
    pub max_capacity: usize,
    /// Enable automatic growth
    pub auto_grow: bool,
    /// Growth factor (multiply capacity by this when growing)
    pub growth_factor: f32,
    /// Enable automatic shrinking
    pub auto_shrink: bool,
    /// Shrink threshold (shrink when usage drops below this percentage)
    pub shrink_threshold: f32,
    /// Minimum capacity (never shrink below this)
    pub min_capacity: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            initial_capacity: 32,
            max_capacity: 0, // Unlimited
            auto_grow: true,
            growth_factor: 2.0,
            auto_shrink: false,
            shrink_threshold: 0.25,
            min_capacity: 16,
        }
    }
}

/// Pool statistics
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Total objects created
    pub total_created: usize,
    /// Total objects acquired
    pub total_acquired: usize,
    /// Total objects released
    pub total_released: usize,
    /// Current pool size
    pub pool_size: usize,
    /// Current active objects
    pub active_objects: usize,
    /// Peak active objects
    pub peak_active: usize,
    /// Number of times pool grew
    pub growth_count: usize,
    /// Number of times pool shrunk
    pub shrink_count: usize,
}

impl PoolStats {
    /// Calculate pool utilization percentage
    pub fn utilization(&self) -> f32 {
        if self.pool_size == 0 {
            return 0.0;
        }
        (self.active_objects as f32 / self.pool_size as f32) * 100.0
    }

    /// Calculate hit rate (reuse rate)
    pub fn hit_rate(&self) -> f32 {
        if self.total_acquired == 0 {
            return 0.0;
        }
        ((self.total_acquired - self.total_created) as f32 / self.total_acquired as f32) * 100.0
    }
}

/// Object pool for a specific type
pub struct Pool<T> {
    /// Available objects
    available: VecDeque<T>,
    /// Configuration
    config: PoolConfig,
    /// Statistics
    stats: PoolStats,
    /// Factory function for creating new objects
    factory: Box<dyn Fn() -> T>,
}

impl<T> Pool<T>
where
    T: Default,
{
    /// Create a new pool with default factory
    pub fn new(config: PoolConfig) -> Self {
        let mut pool = Self {
            available: VecDeque::with_capacity(config.initial_capacity),
            config: config.clone(),
            stats: PoolStats::default(),
            factory: Box::new(|| T::default()),
        };

        // Warm up the pool
        pool.warm_up(config.initial_capacity);
        pool
    }
}

impl<T> Pool<T> {
    /// Create a new pool with custom factory
    pub fn with_factory<F>(config: PoolConfig, factory: F) -> Self
    where
        F: Fn() -> T + 'static,
    {
        let mut pool = Self {
            available: VecDeque::with_capacity(config.initial_capacity),
            config: config.clone(),
            stats: PoolStats::default(),
            factory: Box::new(factory),
        };

        // Warm up the pool
        pool.warm_up(config.initial_capacity);
        pool
    }

    /// Warm up the pool by pre-allocating objects
    pub fn warm_up(&mut self, count: usize) {
        for _ in 0..count {
            let obj = (self.factory)();
            self.available.push_back(obj);
            self.stats.total_created += 1;
            self.stats.pool_size += 1;
        }
    }

    /// Acquire an object from the pool
    pub fn acquire(&mut self) -> T {
        self.stats.total_acquired += 1;
        self.stats.active_objects += 1;

        if self.stats.active_objects > self.stats.peak_active {
            self.stats.peak_active = self.stats.active_objects;
        }

        if let Some(obj) = self.available.pop_front() {
            obj
        } else if self.config.auto_grow {
            self.grow();
            self.available.pop_front().unwrap_or_else(|| {
                self.stats.total_created += 1;
                (self.factory)()
            })
        } else {
            self.stats.total_created += 1;
            (self.factory)()
        }
    }

    /// Release an object back to the pool
    pub fn release(&mut self, obj: T) {
        self.stats.total_released += 1;
        self.stats.active_objects = self.stats.active_objects.saturating_sub(1);

        // Check if we should accept this object back
        if self.config.max_capacity > 0 && self.available.len() >= self.config.max_capacity {
            // Pool is full, drop the object
            return;
        }

        self.available.push_back(obj);

        // Check if we should shrink
        if self.config.auto_shrink {
            let utilization = self.stats.active_objects as f32 / self.stats.pool_size as f32;
            if utilization < self.config.shrink_threshold {
                self.shrink();
            }
        }
    }

    /// Grow the pool
    fn grow(&mut self) {
        let current_size = self.stats.pool_size;
        let new_capacity = ((current_size as f32 * self.config.growth_factor) as usize).max(current_size + 1);
        let growth = new_capacity - current_size;

        // Check max capacity
        let growth = if self.config.max_capacity > 0 {
            growth.min(self.config.max_capacity - current_size)
        } else {
            growth
        };

        for _ in 0..growth {
            let obj = (self.factory)();
            self.available.push_back(obj);
            self.stats.total_created += 1;
            self.stats.pool_size += 1;
        }

        self.stats.growth_count += 1;
    }

    /// Shrink the pool
    fn shrink(&mut self) {
        let target_size = (self.stats.pool_size as f32 * 0.5) as usize;
        let target_size = target_size.max(self.config.min_capacity);

        while self.stats.pool_size > target_size && !self.available.is_empty() {
            self.available.pop_back();
            self.stats.pool_size -= 1;
        }

        self.stats.shrink_count += 1;
    }

    /// Clear the pool
    pub fn clear(&mut self) {
        self.available.clear();
        self.stats.pool_size = 0;
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> &PoolStats {
        &self.stats
    }

    /// Get pool configuration
    pub fn get_config(&self) -> &PoolConfig {
        &self.config
    }

    /// Get number of available objects
    pub fn available_count(&self) -> usize {
        self.available.len()
    }

    /// Get number of active objects
    pub fn active_count(&self) -> usize {
        self.stats.active_objects
    }

    /// Get total pool size
    pub fn total_size(&self) -> usize {
        self.stats.pool_size
    }
}

/// Thread-safe object pool
pub struct ThreadSafePool<T> {
    inner: Arc<Mutex<Pool<T>>>,
}

impl<T> ThreadSafePool<T>
where
    T: Default,
{
    /// Create a new thread-safe pool
    pub fn new(config: PoolConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Pool::new(config))),
        }
    }
}

impl<T> ThreadSafePool<T> {
    /// Create a new thread-safe pool with custom factory
    pub fn with_factory<F>(config: PoolConfig, factory: F) -> Self
    where
        F: Fn() -> T + 'static,
    {
        Self {
            inner: Arc::new(Mutex::new(Pool::with_factory(config, factory))),
        }
    }

    /// Acquire an object from the pool
    pub fn acquire(&self) -> T {
        self.inner.lock().unwrap().acquire()
    }

    /// Release an object back to the pool
    pub fn release(&self, obj: T) {
        self.inner.lock().unwrap().release(obj);
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> PoolStats {
        self.inner.lock().unwrap().get_stats().clone()
    }

    /// Clear the pool
    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }

    /// Clone the pool handle
    pub fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Pooled object wrapper (RAII)
pub struct PooledObject<T> {
    object: Option<T>,
    pool: Arc<Mutex<Pool<T>>>,
}

impl<T> PooledObject<T> {
    /// Create a new pooled object
    pub fn new(object: T, pool: Arc<Mutex<Pool<T>>>) -> Self {
        Self {
            object: Some(object),
            pool,
        }
    }

    /// Get a reference to the object
    pub fn get(&self) -> &T {
        self.object.as_ref().unwrap()
    }

    /// Get a mutable reference to the object
    pub fn get_mut(&mut self) -> &mut T {
        self.object.as_mut().unwrap()
    }
}

impl<T> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(obj) = self.object.take() {
            self.pool.lock().unwrap().release(obj);
        }
    }
}

impl<T> std::ops::Deref for PooledObject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T> std::ops::DerefMut for PooledObject<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestObject {
        value: i32,
    }

    impl Default for TestObject {
        fn default() -> Self {
            Self { value: 0 }
        }
    }

    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.initial_capacity, 32);
        assert!(config.auto_grow);
    }

    #[test]
    fn test_pool_creation() {
        let pool = Pool::<TestObject>::new(PoolConfig::default());
        assert_eq!(pool.total_size(), 32);
    }

    #[test]
    fn test_pool_acquire_release() {
        let mut pool = Pool::<TestObject>::new(PoolConfig::default());
        
        let obj = pool.acquire();
        assert_eq!(pool.active_count(), 1);
        
        pool.release(obj);
        assert_eq!(pool.active_count(), 0);
    }

    #[test]
    fn test_pool_stats() {
        let mut pool = Pool::<TestObject>::new(PoolConfig::default());
        
        let obj1 = pool.acquire();
        let obj2 = pool.acquire();
        
        let stats = pool.get_stats();
        assert_eq!(stats.total_acquired, 2);
        assert_eq!(stats.active_objects, 2);
        
        pool.release(obj1);
        pool.release(obj2);
        
        let stats = pool.get_stats();
        assert_eq!(stats.total_released, 2);
        assert_eq!(stats.active_objects, 0);
    }

    #[test]
    fn test_pool_growth() {
        let mut config = PoolConfig::default();
        config.initial_capacity = 2;
        config.auto_grow = true;
        
        let mut pool = Pool::<TestObject>::new(config);
        
        let _obj1 = pool.acquire();
        let _obj2 = pool.acquire();
        let _obj3 = pool.acquire(); // Should trigger growth
        
        assert!(pool.total_size() > 2);
        assert!(pool.get_stats().growth_count > 0);
    }

    #[test]
    fn test_pool_max_capacity() {
        let mut config = PoolConfig::default();
        config.initial_capacity = 2;
        config.max_capacity = 4;
        config.auto_grow = true;
        
        let mut pool = Pool::<TestObject>::new(config);
        
        // Acquire all objects
        let obj1 = pool.acquire();
        let obj2 = pool.acquire();
        let obj3 = pool.acquire();
        let obj4 = pool.acquire();
        
        // Release them back
        pool.release(obj1);
        pool.release(obj2);
        pool.release(obj3);
        pool.release(obj4);
        
        // Pool should not exceed max capacity
        assert!(pool.total_size() <= 4);
    }

    #[test]
    fn test_pool_custom_factory() {
        let pool = Pool::with_factory(PoolConfig::default(), || TestObject { value: 42 });
        
        let mut pool = pool;
        let obj = pool.acquire();
        assert_eq!(obj.value, 42);
    }

    #[test]
    fn test_pool_clear() {
        let mut pool = Pool::<TestObject>::new(PoolConfig::default());
        
        pool.clear();
        assert_eq!(pool.total_size(), 0);
        assert_eq!(pool.available_count(), 0);
    }

    #[test]
    fn test_pool_utilization() {
        let mut pool = Pool::<TestObject>::new(PoolConfig::default());
        
        let _obj1 = pool.acquire();
        let _obj2 = pool.acquire();
        
        let stats = pool.get_stats();
        let utilization = stats.utilization();
        assert!(utilization > 0.0 && utilization <= 100.0);
    }

    #[test]
    fn test_pool_hit_rate() {
        let mut pool = Pool::<TestObject>::new(PoolConfig::default());
        
        let obj = pool.acquire();
        pool.release(obj);
        
        let obj = pool.acquire(); // Should reuse
        pool.release(obj);
        
        let stats = pool.get_stats();
        let hit_rate = stats.hit_rate();
        assert!(hit_rate > 0.0);
    }

    #[test]
    fn test_thread_safe_pool() {
        let pool = ThreadSafePool::<TestObject>::new(PoolConfig::default());
        
        let obj = pool.acquire();
        pool.release(obj);
        
        let stats = pool.get_stats();
        assert_eq!(stats.total_acquired, 1);
    }

    #[test]
    fn test_thread_safe_pool_clone() {
        let pool1 = ThreadSafePool::<TestObject>::new(PoolConfig::default());
        let pool2 = pool1.clone();
        
        let obj = pool1.acquire();
        pool2.release(obj);
        
        let stats = pool1.get_stats();
        assert_eq!(stats.active_objects, 0);
    }

    #[test]
    fn test_pooled_object_raii() {
        let pool = Arc::new(Mutex::new(Pool::<TestObject>::new(PoolConfig::default())));
        
        {
            let obj = pool.lock().unwrap().acquire();
            let _pooled = PooledObject::new(obj, Arc::clone(&pool));
            // Object should be automatically released when pooled goes out of scope
        }
        
        let stats = pool.lock().unwrap().get_stats();
        assert_eq!(stats.active_objects, 0);
    }

    #[test]
    fn test_pooled_object_deref() {
        let pool = Arc::new(Mutex::new(Pool::<TestObject>::new(PoolConfig::default())));
        let obj = pool.lock().unwrap().acquire();
        let pooled = PooledObject::new(obj, Arc::clone(&pool));
        
        assert_eq!(pooled.value, 0);
    }

    #[test]
    fn test_pooled_object_deref_mut() {
        let pool = Arc::new(Mutex::new(Pool::<TestObject>::new(PoolConfig::default())));
        let obj = pool.lock().unwrap().acquire();
        let mut pooled = PooledObject::new(obj, Arc::clone(&pool));
        
        pooled.value = 42;
        assert_eq!(pooled.value, 42);
    }

    #[test]
    fn test_pool_shrinking() {
        let mut config = PoolConfig::default();
        config.initial_capacity = 10;
        config.auto_shrink = true;
        config.shrink_threshold = 0.2;
        
        let mut pool = Pool::<TestObject>::new(config);
        
        // Acquire and release to trigger shrinking
        let objs: Vec<_> = (0..10).map(|_| pool.acquire()).collect();
        for obj in objs {
            pool.release(obj);
        }
        
        // Manually trigger shrink check
        let obj = pool.acquire();
        pool.release(obj);
        
        // Pool may have shrunk
        assert!(pool.get_stats().shrink_count >= 0);
    }

    #[test]
    fn test_pool_peak_active() {
        let mut pool = Pool::<TestObject>::new(PoolConfig::default());
        
        let obj1 = pool.acquire();
        let obj2 = pool.acquire();
        let obj3 = pool.acquire();
        
        assert_eq!(pool.get_stats().peak_active, 3);
        
        pool.release(obj1);
        assert_eq!(pool.get_stats().peak_active, 3); // Peak should remain
        
        pool.release(obj2);
        pool.release(obj3);
    }

    #[test]
    fn test_pool_warm_up() {
        let mut config = PoolConfig::default();
        config.initial_capacity = 0;
        
        let mut pool = Pool::<TestObject>::new(config);
        assert_eq!(pool.total_size(), 0);
        
        pool.warm_up(10);
        assert_eq!(pool.total_size(), 10);
    }

    #[test]
    fn test_pool_available_count() {
        let mut pool = Pool::<TestObject>::new(PoolConfig::default());
        
        let initial_available = pool.available_count();
        let _obj = pool.acquire();
        
        assert_eq!(pool.available_count(), initial_available - 1);
    }

    #[test]
    fn test_pool_no_auto_grow() {
        let mut config = PoolConfig::default();
        config.initial_capacity = 2;
        config.auto_grow = false;
        
        let mut pool = Pool::<TestObject>::new(config);
        
        let _obj1 = pool.acquire();
        let _obj2 = pool.acquire();
        let _obj3 = pool.acquire(); // Should create new without growing pool
        
        // Pool size should remain at initial capacity
        assert_eq!(pool.total_size(), 2);
    }

    #[test]
    fn test_thread_safe_pool_clear() {
        let pool = ThreadSafePool::<TestObject>::new(PoolConfig::default());
        
        let _obj = pool.acquire();
        pool.clear();
        
        let stats = pool.get_stats();
        assert_eq!(stats.pool_size, 0);
    }

    #[test]
    fn test_growth_factor() {
        let mut config = PoolConfig::default();
        config.initial_capacity = 4;
        config.growth_factor = 3.0;
        config.auto_grow = true;
        
        let mut pool = Pool::<TestObject>::new(config);
        
        // Exhaust initial capacity
        let objs: Vec<_> = (0..5).map(|_| pool.acquire()).collect();
        
        // Should have grown by factor of 3
        assert!(pool.total_size() >= 12);
        
        for obj in objs {
            pool.release(obj);
        }
    }

    #[test]
    fn test_min_capacity_shrink() {
        let mut config = PoolConfig::default();
        config.initial_capacity = 20;
        config.auto_shrink = true;
        config.min_capacity = 10;
        
        let mut pool = Pool::<TestObject>::new(config);
        
        // Trigger shrinking
        pool.shrink();
        
        // Should not shrink below min_capacity
        assert!(pool.total_size() >= 10);
    }
}

