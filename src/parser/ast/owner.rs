//! Ownership of AST storage: a `Program` keeps the arenas its nodes live in alive.
//!
//! The parser allocates `Expression`/`Statement`/`Pattern` nodes in arenas and hands out references. Those
//! references are typed `'static` (the nodes are reachable from `Program<'static>`), so the type system cannot
//! tie a program to the arenas. Instead the program carries an [`AstOwner`]: reference-counted handles to every
//! arena it borrows from. The arenas are freed when the last owner is dropped, never earlier and never later.
//! See `docs/design/parser-arena-ownership.md`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use typed_arena::Arena;

use super::core::{Expression, Pattern, Statement};

/// Counts frees of one arena bundle; a clone outlives the bundle so tests can observe the drop.
#[derive(Clone, Default, Debug)]
pub struct ArenaLedger(Arc<AtomicUsize>);

impl ArenaLedger {
    /// Number of times the bundle was actually freed (0 or 1).
    pub fn freed(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }

    /// Identity of the counter (two ledgers of different bundles never share one).
    pub fn counter(&self) -> *const AtomicUsize {
        Arc::as_ptr(&self.0)
    }
}

/// The three node arenas of one parse.
pub struct AstArenas {
    pub(crate) expr: Arena<Expression<'static>>,
    pub(crate) stmt: Arena<Statement<'static>>,
    pub(crate) pattern: Arena<Pattern<'static>>,
    ledger: ArenaLedger,
}

// SAFETY: `typed_arena::Arena` is `!Sync` only because `alloc(&self)` mutates through a `RefCell`. An
// `AstArenas` is shared (via `Arc`) only after the parser has *sealed* it (`Parser::seal_arenas` swaps in a
// fresh bundle, so the parser never allocates in a bundle a `Program` can see). From then on nobody calls
// `alloc`; the bundle is read-only storage that is dropped when the last `Arc` goes. Moving it to another
// thread is fine because the nodes contain no thread-affine data.
unsafe impl Send for AstArenas {}
unsafe impl Sync for AstArenas {}

impl AstArenas {
    pub fn new() -> Self {
        AstArenas {
            expr: Arena::new(),
            stmt: Arena::new(),
            pattern: Arena::new(),
            ledger: ArenaLedger::default(),
        }
    }

    pub fn ledger(&self) -> ArenaLedger {
        self.ledger.clone()
    }
}

impl Default for AstArenas {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AstArenas {
    fn drop(&mut self) {
        self.ledger.0.fetch_add(1, Ordering::SeqCst);
    }
}

/// Keeps the arenas a program (or any item list built from programs) borrows from alive.
#[derive(Clone, Default)]
pub struct AstOwner(Vec<Arc<AstArenas>>);

impl AstOwner {
    pub fn none() -> Self {
        AstOwner(Vec::new())
    }

    pub fn of(arenas: Arc<AstArenas>) -> Self {
        AstOwner(vec![arenas])
    }

    /// Union of owners, for programs assembled from the items of several programs.
    pub fn merge<'a>(owners: impl IntoIterator<Item = &'a AstOwner>) -> Self {
        let mut all: Vec<Arc<AstArenas>> = Vec::new();
        for o in owners {
            for a in &o.0 {
                if !all.iter().any(|x| Arc::ptr_eq(x, a)) {
                    all.push(a.clone());
                }
            }
        }
        AstOwner(all)
    }
}

// An owner is storage, not value: it never participates in AST equality, hashing or printing.
impl PartialEq for AstOwner {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Eq for AstOwner {}
impl std::hash::Hash for AstOwner {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}
impl std::fmt::Debug for AstOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AstOwner({} arenas)", self.0.len())
    }
}
