//! Arenas for per-frame, per-request and per-phase allocation (`std::arena`, ADR-0016).
//!
//! Two shapes:
//!
//! * [`Arena<T>`]: a typed pool. `alloc` returns a [`Handle<T>`] (two `u32`s, `Copy`); values are stored
//!   contiguously; `reset` drops every value and invalidates every handle in O(1) bookkeeping (plus the drops,
//!   which are free for types without destructors). Handles carry the arena generation; debug builds check it
//!   on every access and panic on a stale or foreign handle. Release builds skip the check: a stale handle then
//!   reads another live value or panics on bounds, it is a logic error but never undefined behaviour.
//! * [`Scratch`]: an untyped bump allocator for transient strings and lists (`str`, `string`, `list`). The
//!   borrow checker ties every scratch value to `&Scratch`, so `reset(&mut self)` cannot run while one is alive;
//!   Windjammer's escape check (ADR-0016) reports the same mistake earlier, in Windjammer terms.
//!
//! Go/JS backends and the interpreter implement the same API on their heap; only the cost model differs.

use std::fmt;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_ARENA_ID: AtomicU32 = AtomicU32::new(1);

/// Copy index into an [`Arena<T>`]. Valid until the arena is reset.
pub struct Handle<T> {
    index: u32,
    generation: u32,
    arena_id: u32,
    _t: PhantomData<fn() -> T>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Handle<T> {}
impl<T> PartialEq for Handle<T> {
    fn eq(&self, o: &Self) -> bool {
        (self.index, self.generation, self.arena_id) == (o.index, o.generation, o.arena_id)
    }
}
impl<T> Eq for Handle<T> {}
impl<T> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        (self.index, self.generation, self.arena_id).hash(h)
    }
}
impl<T> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Handle({}@g{})", self.index, self.generation)
    }
}

impl<T> Handle<T> {
    /// Position in allocation order (0-based) within the arena's current generation.
    pub fn index(&self) -> i64 {
        self.index as i64
    }
}

/// Typed arena: contiguous storage, `Copy` handles, whole-arena reset.
pub struct Arena<T> {
    items: Vec<T>,
    generation: u32,
    id: u32,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Arena { items: Vec::with_capacity(capacity), generation: 0, id: NEXT_ARENA_ID.fetch_add(1, Ordering::Relaxed) }
    }

    #[inline]
    pub fn alloc(&mut self, value: T) -> Handle<T> {
        let index = self.items.len();
        assert!(index < u32::MAX as usize, "arena is full (2^32 - 1 values)");
        self.items.push(value);
        Handle { index: index as u32, generation: self.generation, arena_id: self.id, _t: PhantomData }
    }

    #[inline]
    fn check(&self, h: Handle<T>) {
        if cfg!(debug_assertions) {
            assert!(h.arena_id == self.id, "arena handle used with a different arena");
            assert!(
                h.generation == self.generation,
                "stale arena handle: allocated before reset (generation {}, arena is at generation {})",
                h.generation,
                self.generation
            );
        }
    }

    #[inline]
    pub fn get(&self, h: Handle<T>) -> &T {
        self.check(h);
        &self.items[h.index as usize]
    }

    #[inline]
    pub fn get_mut(&mut self, h: Handle<T>) -> &mut T {
        self.check(h);
        &mut self.items[h.index as usize]
    }

    /// Always-checked lookup (release builds too): `None` for a stale or foreign handle.
    pub fn try_get(&self, h: Handle<T>) -> Option<&T> {
        if h.arena_id != self.id || h.generation != self.generation {
            return None;
        }
        self.items.get(h.index as usize)
    }

    /// Drop every value and invalidate every handle. Keeps the capacity: the next frame does not allocate.
    pub fn reset(&mut self) {
        self.items.clear();
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn len(&self) -> i64 {
        self.items.len() as i64
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn capacity(&self) -> i64 {
        self.items.capacity() as i64
    }

    pub fn generation(&self) -> i64 {
        self.generation as i64
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.items.iter_mut()
    }
}

impl<T> Index<Handle<T>> for Arena<T> {
    type Output = T;
    #[inline]
    fn index(&self, h: Handle<T>) -> &T {
        self.get(h)
    }
}

impl<T> IndexMut<Handle<T>> for Arena<T> {
    #[inline]
    fn index_mut(&mut self, h: Handle<T>) -> &mut T {
        self.get_mut(h)
    }
}

impl<'a, T> IntoIterator for &'a Arena<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Arena<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter_mut()
    }
}

impl<T: fmt::Debug> fmt::Debug for Arena<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Arena").field("len", &self.items.len()).field("generation", &self.generation).finish()
    }
}

/// Arena-backed growable string (`Scratch::string`).
pub type ScratchString<'s> = bumpalo::collections::String<'s>;
/// Arena-backed growable list (`Scratch::list`).
pub type ScratchList<'s, T> = bumpalo::collections::Vec<'s, T>;

/// Untyped bump allocator for transient strings and lists; `reset` frees everything at once.
#[derive(Default)]
pub struct Scratch {
    bump: bumpalo::Bump,
    generation: u32,
}

impl Scratch {
    pub fn new() -> Self {
        Scratch { bump: bumpalo::Bump::new(), generation: 0 }
    }

    pub fn with_capacity(bytes: usize) -> Self {
        Scratch { bump: bumpalo::Bump::with_capacity(bytes), generation: 0 }
    }

    /// Copy `s` into the scratch arena.
    #[inline]
    pub fn str<'s>(&'s self, s: &str) -> &'s str {
        self.bump.alloc_str(s)
    }

    /// An empty growable string in the scratch arena.
    #[inline]
    pub fn string(&self) -> ScratchString<'_> {
        ScratchString::new_in(&self.bump)
    }

    /// An empty growable list in the scratch arena.
    #[inline]
    pub fn list<T>(&self) -> ScratchList<'_, T> {
        ScratchList::new_in(&self.bump)
    }

    /// Place a value without a destructor in the arena (destructors would never run).
    #[inline]
    pub fn alloc<T: Copy>(&self, value: T) -> &mut T {
        self.bump.alloc(value)
    }

    /// Free everything allocated since the last reset; keeps the largest chunk for reuse.
    pub fn reset(&mut self) {
        self.bump.reset();
        self.generation = self.generation.wrapping_add(1);
    }

    /// Bytes currently held in chunks (capacity, not live data).
    pub fn allocated_bytes(&self) -> i64 {
        self.bump.allocated_bytes() as i64
    }

    pub fn generation(&self) -> i64 {
        self.generation as i64
    }
}

impl fmt::Debug for Scratch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scratch").field("allocated_bytes", &self.bump.allocated_bytes()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct P {
        x: f32,
        v: f32,
    }

    #[test]
    fn alloc_get_mutate() {
        let mut a = Arena::new();
        let h1 = a.alloc(P { x: 0.0, v: 1.0 });
        let h2 = a.alloc(P { x: 5.0, v: -1.0 });
        a[h1].x += a[h1].v;
        a.get_mut(h2).x += a[h2].v;
        assert_eq!((a[h1].x, a[h2].x), (1.0, 4.0));
        assert_eq!((a.len(), h2.index()), (2, 1));
        assert_eq!(a.iter().map(|p| p.x).sum::<f32>(), 5.0);
    }

    #[test]
    fn reset_invalidates_handles_and_keeps_capacity() {
        let mut a = Arena::with_capacity(4);
        let h = a.alloc(P { x: 1.0, v: 0.0 });
        let cap = a.capacity();
        a.reset();
        assert!(a.is_empty());
        assert_eq!(a.capacity(), cap);
        assert_eq!(a.try_get(h), None, "stale handle must not resolve");
        let h2 = a.alloc(P { x: 2.0, v: 0.0 });
        assert_eq!(a.try_get(h2).map(|p| p.x), Some(2.0));
        assert_ne!(h, h2, "same slot, different generation");
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "stale arena handle")]
    fn debug_build_panics_on_stale_handle() {
        let mut a = Arena::new();
        let h = a.alloc(1u8);
        a.reset();
        a.alloc(2u8);
        let _ = a[h];
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "different arena")]
    fn debug_build_panics_on_foreign_handle() {
        let mut a = Arena::new();
        let mut b = Arena::new();
        let _ = b.alloc(0u8);
        let h = a.alloc(1u8);
        let _ = b[h];
    }

    #[test]
    fn reset_runs_destructors() {
        let rc = std::rc::Rc::new(());
        let mut a = Arena::new();
        for _ in 0..10 {
            a.alloc(rc.clone());
        }
        assert_eq!(std::rc::Rc::strong_count(&rc), 11);
        a.reset();
        assert_eq!(std::rc::Rc::strong_count(&rc), 1);
    }

    #[test]
    fn scratch_strings_and_lists() {
        let mut s = Scratch::new();
        {
            let a = s.str("hello");
            let mut b = s.string();
            b.push_str(a);
            b.push_str(", world");
            let mut l = s.list();
            for i in 0..100 {
                l.push(i * 2);
            }
            assert_eq!(b.as_str(), "hello, world");
            assert_eq!(l.iter().sum::<i32>(), 9900);
            *s.alloc(7i64) += 1;
        }
        let before = s.allocated_bytes();
        s.reset();
        assert_eq!(s.generation(), 1);
        assert!(s.allocated_bytes() <= before, "reset keeps at most the existing chunks");
        // Steady state: the same work after reset needs no new chunk.
        let after_reset = s.allocated_bytes();
        {
            let mut b = s.string();
            b.push_str("hello, world");
        }
        assert_eq!(s.allocated_bytes(), after_reset);
    }
}
