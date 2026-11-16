//! ADR-0004 tasks for compiled programs (docs/concurrency/PLAN.md, stage 2b): `spawn` runs on an OS thread,
//! `async` runs on the current thread's local queue (at the next blocking point: `wait()`, a channel `recv()`,
//! a `select`, a scope's end), `scope { }` is a guard whose drop waits for every task started inside it (also by
//! those tasks) and re-raises the first panic. No async runtime, no dependency: compile time is a feature.
//!
//! The semantics mirror the interpreter's deterministic scheduler (`src/interpreter/sched.rs`): output that depends
//! only on `wait()`, channel receives and scope exit is identical (tests/conformance/concurrency/).

use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

type Payload = Box<dyn Any + Send + 'static>;

enum Slot<T> {
    Running,
    Done(Result<T, Payload>),
    Taken,
}

struct Shared<T> {
    slot: Mutex<Slot<T>>,
    done: Condvar,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// A task a scope waits for.
trait Join: Send + Sync {
    /// Wait until the task has finished; its panic payload if it panicked and nobody took it.
    fn join(&self) -> Option<Payload>;
}

impl<T: Send> Join for Shared<T> {
    fn join(&self) -> Option<Payload> {
        let mut g = lock(&self.slot);
        while matches!(*g, Slot::Running) {
            g = self.done.wait(g).unwrap_or_else(|e| e.into_inner());
        }
        if matches!(*g, Slot::Done(Err(_))) {
            if let Slot::Done(Err(p)) = std::mem::replace(&mut *g, Slot::Taken) {
                return Some(p);
            }
        }
        None
    }
}

#[derive(Default)]
struct Registry {
    tasks: Mutex<Vec<Arc<dyn Join>>>,
}

enum LocalSlot<T> {
    Pending(Option<Box<dyn FnOnce() -> T>>),
    Done(Result<T, Payload>),
    Taken,
}

trait LocalRun {
    fn run(&self);
}

impl<T> LocalRun for RefCell<LocalSlot<T>> {
    fn run(&self) {
        let f = match &mut *self.borrow_mut() {
            LocalSlot::Pending(f) => f.take(),
            _ => None,
        };
        if let Some(f) = f {
            let r = panic::catch_unwind(AssertUnwindSafe(f));
            *self.borrow_mut() = LocalSlot::Done(r);
        }
    }
}

thread_local! {
    /// Open scopes of this thread, innermost last; a thread started inside a scope starts with that scope.
    static SCOPES: RefCell<Vec<Arc<Registry>>> = const { RefCell::new(Vec::new()) };
    /// `async` tasks of this thread that have not run yet, in start order.
    static LOCAL: RefCell<VecDeque<Rc<dyn LocalRun>>> = RefCell::new(VecDeque::new());
}

/// Run every `async` task of this thread that has not run yet (a blocking point).
pub fn run_pending_local() {
    loop {
        let next = LOCAL.with(|q| q.borrow_mut().pop_front());
        match next {
            Some(t) => t.run(),
            None => break,
        }
    }
}

/// A running or finished task: `spawn f()`, `spawn { }`, `async f()`, `async { }`.
pub struct Task<T> {
    inner: Inner<T>,
}

enum Inner<T> {
    Thread(Arc<Shared<T>>),
    Local(Rc<RefCell<LocalSlot<T>>>),
}

impl<T> Clone for Task<T> {
    fn clone(&self) -> Self {
        Task {
            inner: match &self.inner {
                Inner::Thread(s) => Inner::Thread(s.clone()),
                Inner::Local(s) => Inner::Local(s.clone()),
            },
        }
    }
}

const TAKEN: &str = "the value of this task was already taken by an earlier wait()";

impl<T> Task<T> {
    /// Wait for the task and return its value; a panic of the task is re-raised here.
    pub fn wait(&self) -> T {
        match &self.inner {
            Inner::Thread(s) => {
                run_pending_local();
                let mut g = lock(&s.slot);
                while matches!(*g, Slot::Running) {
                    g = s.done.wait(g).unwrap_or_else(|e| e.into_inner());
                }
                match std::mem::replace(&mut *g, Slot::Taken) {
                    Slot::Done(Ok(v)) => v,
                    Slot::Done(Err(p)) => {
                        drop(g);
                        panic::resume_unwind(p)
                    }
                    _ => panic!("{}", TAKEN),
                }
            }
            Inner::Local(s) => {
                s.run();
                let r = std::mem::replace(&mut *s.borrow_mut(), LocalSlot::Taken);
                match r {
                    LocalSlot::Done(Ok(v)) => v,
                    LocalSlot::Done(Err(p)) => panic::resume_unwind(p),
                    _ => panic!("{}", TAKEN),
                }
            }
        }
    }

    /// True once the task has returned or panicked (never waits).
    pub fn is_finished(&self) -> bool {
        match &self.inner {
            Inner::Thread(s) => !matches!(*lock(&s.slot), Slot::Running),
            Inner::Local(s) => !matches!(*s.borrow(), LocalSlot::Pending(_)),
        }
    }
}

/// `spawn f()` / `spawn { }`: run `f` on a new thread; inside a scope, the scope waits for it.
pub fn spawn<T, F>(f: F) -> Task<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let shared = Arc::new(Shared { slot: Mutex::new(Slot::Running), done: Condvar::new() });
    let inherited = SCOPES.with(|s| s.borrow().last().cloned());
    if let Some(reg) = &inherited {
        lock(&reg.tasks).push(shared.clone() as Arc<dyn Join>);
    }
    let out = shared.clone();
    std::thread::spawn(move || {
        if let Some(reg) = inherited {
            SCOPES.with(|s| s.borrow_mut().push(reg));
        }
        let r = panic::catch_unwind(AssertUnwindSafe(f));
        run_pending_local();
        *lock(&out.slot) = Slot::Done(r);
        out.done.notify_all();
    });
    Task { inner: Inner::Thread(shared) }
}

/// `async f()` / `async { }`: run `f` on this thread at its next blocking point.
pub fn local<T: 'static, F: FnOnce() -> T + 'static>(f: F) -> Task<T> {
    let slot: Rc<RefCell<LocalSlot<T>>> = Rc::new(RefCell::new(LocalSlot::Pending(Some(Box::new(f)))));
    LOCAL.with(|q| q.borrow_mut().push_back(slot.clone() as Rc<dyn LocalRun>));
    Task { inner: Inner::Local(slot) }
}

/// `scope { }`: `let _scope = Scope::enter();` at the start of the block; dropping it (at the block's end, also on
/// `return`, `?` or `break`) waits for every task started inside and re-raises the first panic.
pub struct Scope {
    reg: Arc<Registry>,
}

impl Scope {
    pub fn enter() -> Scope {
        let reg = Arc::new(Registry::default());
        SCOPES.with(|s| s.borrow_mut().push(reg.clone()));
        Scope { reg }
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        SCOPES.with(|s| {
            s.borrow_mut().pop();
        });
        run_pending_local();
        let mut first: Option<Payload> = None;
        let mut i = 0;
        loop {
            let next = lock(&self.reg.tasks).get(i).cloned();
            let Some(t) = next else { break };
            i += 1;
            if let Some(p) = t.join() {
                first.get_or_insert(p);
            }
        }
        if let Some(p) = first {
            if !std::thread::panicking() {
                panic::resume_unwind(p);
            }
        }
    }
}

/// A source of a `select` arm.
pub trait Selectable {
    /// A value is ready now (taking it does not wait).
    fn ready(&self) -> bool;
    /// It can never become ready (a closed, empty channel).
    fn closed(&self) -> bool {
        false
    }
}

impl<T> Selectable for Task<T> {
    fn ready(&self) -> bool {
        self.is_finished()
    }
}

/// `_ = timeout(d)`: ready once `d` has passed since the `select` started.
pub struct Deadline(Instant);

impl Deadline {
    pub fn after<D: IntoTimeout>(d: D) -> Deadline {
        Deadline(Instant::now() + d.into_timeout())
    }
}

/// What `timeout(d)` accepts: `std::time::Duration`, and the runtime's `time::Duration` when it is chrono's.
pub trait IntoTimeout {
    fn into_timeout(self) -> Duration;
}

impl IntoTimeout for Duration {
    fn into_timeout(self) -> Duration {
        self
    }
}

#[cfg(feature = "datetime")]
impl IntoTimeout for chrono::Duration {
    fn into_timeout(self) -> Duration {
        self.to_std().unwrap_or(Duration::ZERO)
    }
}

impl Selectable for Deadline {
    fn ready(&self) -> bool {
        Instant::now() >= self.0
    }
}

/// `select { }`: the index of the first ready source (source order), or `sources.len()` for `else` when nothing is
/// ready and the select has one. Panics when no source can ever become ready.
pub fn select(sources: &[&dyn Selectable], has_else: bool) -> usize {
    loop {
        run_pending_local();
        if let Some(i) = sources.iter().position(|s| s.ready()) {
            return i;
        }
        if has_else {
            return sources.len();
        }
        if sources.iter().all(|s| s.closed()) {
            panic!("every channel of this `select` is closed");
        }
        std::thread::sleep(Duration::from_micros(200));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn scope_waits_for_nested_tasks() {
        let n = Arc::new(AtomicUsize::new(0));
        {
            let _s = Scope::enter();
            for _ in 0..3 {
                let n = n.clone();
                spawn(move || {
                    let m = n.clone();
                    spawn(move || m.fetch_add(1, Ordering::SeqCst));
                    n.fetch_add(1, Ordering::SeqCst)
                });
            }
        }
        assert_eq!(n.load(Ordering::SeqCst), 6);
    }

    #[test]
    fn wait_returns_values_and_local_tasks_run_at_blocking_points() {
        let t = spawn(|| 21 * 2);
        let l = local(|| 5);
        assert!(!l.is_finished());
        assert_eq!(t.wait(), 42);
        assert!(l.is_finished());
        assert_eq!(l.wait(), 5);
    }

    #[test]
    fn a_task_panic_reaches_the_waiter() {
        let t = spawn(|| -> i32 { panic!("boom") });
        let r = panic::catch_unwind(AssertUnwindSafe(|| t.wait()));
        assert!(r.is_err());
    }
}
