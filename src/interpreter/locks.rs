//! Lock and borrow state for `Mutex`, `RwLock` and `RefCell`.
//!
//! A guard is a `Value::Ref` whose place carries a [`GuardToken`]. The token lives exactly as long
//! as some copy of the guard value does, so scope exit releases the lock/borrow (RAII) without any
//! drop bookkeeping; `CellState` only keeps weak handles and counts the live ones.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

impl std::fmt::Debug for GuardToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<guard>")
    }
}

pub struct GuardToken {
    state: Weak<CellState>,
    pub exclusive: bool,
    /// Lock guards poison on panic; `RefCell` borrows do not.
    pub poisons: Cell<bool>,
    /// `Condvar::wait` releases the lock while the guard value is still alive.
    pub released: Cell<bool>,
}

impl std::fmt::Debug for CellState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<lock state>")
    }
}

#[derive(Default)]
pub struct CellState {
    holders: RefCell<Vec<Weak<GuardToken>>>,
    /// A `Mutex`/`RwLock` whose guard was held by a panicking thread (Rust's lock poisoning).
    pub poisoned: Cell<bool>,
}

impl GuardToken {
    pub fn state(&self) -> Option<Rc<CellState>> {
        self.state.upgrade()
    }
}

impl CellState {
    /// (shared holders, exclusive holder present) among live, unreleased guards.
    fn live(&self) -> (usize, bool) {
        let mut h = self.holders.borrow_mut();
        h.retain(|w| w.strong_count() > 0);
        let (mut shared, mut excl) = (0, false);
        for t in h.iter().filter_map(|w| w.upgrade()) {
            if t.released.get() {
                continue;
            }
            if t.exclusive {
                excl = true;
            } else {
                shared += 1;
            }
        }
        (shared, excl)
    }

    pub fn can_acquire(&self, exclusive: bool) -> bool {
        let (shared, excl) = self.live();
        if exclusive {
            shared == 0 && !excl
        } else {
            !excl
        }
    }

    pub fn exclusively_held(&self) -> bool {
        self.live().1
    }

    pub fn any_held(&self) -> bool {
        let (s, e) = self.live();
        s > 0 || e
    }

    pub fn acquire(self: &Rc<Self>, exclusive: bool) -> Rc<GuardToken> {
        let t = Rc::new(GuardToken { state: Rc::downgrade(self), exclusive, poisons: Cell::new(false), released: Cell::new(false) });
        self.holders.borrow_mut().push(Rc::downgrade(&t));
        t
    }
}
