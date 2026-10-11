//! Deterministic cooperative scheduler for threads and tasks.
//!
//! Every spawned thread gets its own OS thread (a private native stack, so the tree walker can
//! recurse), but only one runs at a time: a baton (`Mutex<usize>` + `Condvar`) names the running
//! task and the interpreter state that belongs to a task (scope, frames, depth...) is swapped on
//! every switch. Tasks run until they block (join, recv on an empty channel, a held lock, sleep...),
//! so a run is a pure function of the program and the schedule seed:
//!
//! * seed 0: FIFO order, a task is never preempted; `main` keeps running after `spawn`.
//! * seed N != 0: at synchronisation points (lock, atomic op, send, recv, spawn, join) a seeded
//!   xorshift decides whether to switch to another runnable task, exploring other interleavings.
//!
//! `sleep` and timeouts use a virtual clock: when every task is blocked the clock jumps to the
//! earliest deadline, so programs that sleep do not make the interpreter slow, and the order of
//! sleep-ordered events is exact. If nothing can run and nothing is waiting on the clock the program
//! has deadlocked, which is reported as an error (real Rust would hang).
//!
//! The interpreter is not `Send`; handing `*mut Interpreter` between the task threads is sound
//! because the baton guarantees exactly one thread touches it at a time (the mutex hand-off orders
//! the memory accesses).

use super::engine::{Interpreter, Scope};
use super::error::{ErrorKind, RuntimeError, R};
use super::value::*;
use crate::parser::Type;
use crate::source_map::Location;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};

const TASK_STACK: usize = 256 << 20;
const TASK_STACK_HEADROOM: usize = 32 << 20;

pub(crate) struct Ctx {
    scope: Scope,
    frames: Vec<Scope>,
    depth: usize,
    backtrace: Vec<(Rc<str>, Option<&'static Location>)>,
    cur: Option<&'static Location>,
    stack_guard: Option<(usize, usize)>,
    type_hint: Option<Type>,
    expected: Option<(usize, Type)>,
}

impl Ctx {
    fn fresh() -> Ctx {
        Ctx {
            scope: Scope::new(0),
            frames: Vec::new(),
            depth: 0,
            backtrace: Vec::new(),
            cur: None,
            stack_guard: None,
            type_hint: None,
            expected: None,
        }
    }
}

/// What a blocked task is waiting for.
#[derive(Clone)]
pub(crate) enum Wait {
    Ready,
    Join(usize),
    /// Until a message is queued or every sender is gone (or the deadline passes).
    Recv(Rc<super::builtins::channel::Chan>, Option<u64>),
    /// Until the bounded queue has room (or the receiver is gone).
    Room(Rc<super::builtins::channel::Chan>),
    /// Until the queue is empty (rendezvous channels).
    Drained(Rc<super::builtins::channel::Chan>),
    Lock(Rc<super::locks::CellState>, bool),
    Sleep(u64),
    /// `Condvar` ticket, with an optional deadline.
    Cond(Rc<super::builtins::sync::CondState>, u64, Option<u64>),
    Barrier(Rc<super::builtins::sync::BarrierState>, u64),
}

pub(crate) struct Task {
    pub name: Option<String>,
    pub done: bool,
    wait: Wait,
    ctx: Option<Ctx>,
    /// `Ok(value)` or `Err(panic message)`.
    pub result: Option<Result<Value, String>>,
}

struct Baton {
    current: Mutex<usize>,
    cv: Condvar,
}

impl Baton {
    fn set(&self, t: usize) {
        *self.current.lock().unwrap() = t;
        self.cv.notify_all();
    }
    fn wait_for(&self, me: usize) {
        let mut g = self.current.lock().unwrap();
        while *g != me {
            g = self.cv.wait(g).unwrap();
        }
    }
}

pub(crate) struct Sched {
    pub tasks: Vec<Task>,
    pub current: usize,
    pub seed: u64,
    rng: u64,
    /// Virtual nanoseconds since program start.
    pub clock: u64,
    baton: Arc<Baton>,
    /// An error raised on a task thread that must end the whole program (unsupported feature, exit).
    fatal: Option<RuntimeError>,
    /// Fixed wall clock, no real time (see `builtins/time.rs`).
    pub deterministic: bool,
}

impl Sched {
    pub fn new() -> Sched {
        Sched {
            tasks: vec![Task { name: Some("main".into()), done: false, wait: Wait::Ready, ctx: None, result: None }],
            current: 0,
            seed: 0,
            rng: 0,
            clock: 0,
            baton: Arc::new(Baton { current: Mutex::new(0), cv: Condvar::new() }),
            fatal: None,
            deterministic: false,
        }
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
        self.rng = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    }

    fn next_rand(&mut self) -> u64 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        x
    }

    fn ready(&self, w: &Wait) -> bool {
        match w {
            Wait::Ready => true,
            Wait::Join(t) => self.tasks[*t].done,
            Wait::Recv(c, dl) => c.has_message() || !c.senders_alive() || dl.map_or(false, |d| self.clock >= d),
            Wait::Room(c) => c.has_room() || !c.receiver_alive(),
            Wait::Drained(c) => !c.has_message() || !c.receiver_alive(),
            Wait::Lock(st, excl) => st.can_acquire(*excl),
            Wait::Sleep(u) => self.clock >= *u,
            Wait::Cond(cv, ticket, dl) => cv.woken(*ticket) || dl.map_or(false, |d| self.clock >= d),
            Wait::Barrier(b, gen) => b.generation() > *gen,
        }
    }

    fn deadline(w: &Wait) -> Option<u64> {
        match w {
            Wait::Sleep(u) => Some(*u),
            Wait::Recv(_, d) | Wait::Cond(_, _, d) => *d,
            _ => None,
        }
    }

    fn runnable(&self, except: usize) -> Vec<usize> {
        let n = self.tasks.len();
        (1..=n)
            .map(|k| (except + k) % n)
            .filter(|&t| t != except && !self.tasks[t].done && self.ready(&self.tasks[t].wait))
            .collect()
    }

    /// The next task to run when `me` cannot continue. Advances the virtual clock when everything
    /// is waiting for time. `None` = deadlock.
    fn pick(&mut self, me: usize, me_ready: bool) -> Option<usize> {
        loop {
            let mut cands = self.runnable(me);
            if me_ready {
                cands.push(me);
            }
            if !cands.is_empty() {
                // seed 0: first in FIFO order after `me`; otherwise a seeded choice
                return Some(if self.seed == 0 || cands.len() == 1 {
                    cands[0]
                } else {
                    let r = self.next_rand() as usize;
                    cands[r % cands.len()]
                });
            }
            let next = self
                .tasks
                .iter()
                .enumerate()
                .filter(|(_, t)| !t.done)
                .filter_map(|(_, t)| Self::deadline(&t.wait))
                .filter(|d| *d > self.clock)
                .min();
            match next {
                Some(d) => self.clock = d,
                None => return None,
            }
        }
    }
}

impl Sched {
    /// Scopes and results held by tasks that are not currently running (reachability roots).
    pub(crate) fn parked_roots(&self) -> (Vec<&Scope>, Vec<&Value>) {
        let mut scopes = Vec::new();
        let mut vals = Vec::new();
        for t in &self.tasks {
            if let Some(c) = &t.ctx {
                scopes.push(&c.scope);
                scopes.extend(c.frames.iter());
            }
            if let Some(Ok(v)) = &t.result {
                vals.push(v);
            }
        }
        (scopes, vals)
    }
}

struct SendPtr(*mut Interpreter);
unsafe impl Send for SendPtr {}
struct SendVal(Value);
unsafe impl Send for SendVal {}

impl Interpreter {
    fn take_ctx(&mut self) -> Ctx {
        Ctx {
            scope: std::mem::replace(&mut self.scope, Scope::new(0)),
            frames: std::mem::take(&mut self.frames),
            depth: std::mem::take(&mut self.depth),
            backtrace: std::mem::take(&mut self.backtrace),
            cur: self.cur.take(),
            stack_guard: self.stack_guard.take(),
            type_hint: self.type_hint.take(),
            expected: self.expected.take(),
        }
    }

    fn put_ctx(&mut self, c: Ctx) {
        self.scope = c.scope;
        self.frames = c.frames;
        self.depth = c.depth;
        self.backtrace = c.backtrace;
        self.cur = c.cur;
        self.stack_guard = c.stack_guard;
        self.type_hint = c.type_hint;
        self.expected = c.expected;
    }

    /// Hand the baton to `target` and wait until it comes back to the calling task.
    fn switch_to(&mut self, target: usize) -> R<()> {
        let me = self.sched.current;
        if target == me {
            return Ok(());
        }
        let mine = self.take_ctx();
        self.sched.tasks[me].ctx = Some(mine);
        let theirs = self.sched.tasks[target].ctx.take().unwrap_or_else(Ctx::fresh);
        self.put_ctx(theirs);
        self.sched.current = target;
        let baton = self.sched.baton.clone();
        let p = self as *mut Interpreter;
        baton.set(target);
        baton.wait_for(me);
        // SAFETY: only the task holding the baton runs; `p` is this interpreter.
        let it = unsafe { &mut *p };
        match it.sched.fatal.clone() {
            Some(e) => Err(e.into()),
            None => Ok(()),
        }
    }

    /// Block the current task until `w` holds, running other tasks meanwhile.
    pub(crate) fn block_on(&mut self, w: Wait) -> R<()> {
        loop {
            if self.sched.ready(&w) {
                return Ok(());
            }
            let me = self.sched.current;
            self.sched.tasks[me].wait = w.clone();
            let next = self.sched.pick(me, false);
            let r = match next {
                Some(t) => self.switch_to(t),
                None => Err(RuntimeError::new(
                    ErrorKind::Semantic,
                    "deadlock: every thread is blocked and none is waiting for a timer (a compiled program would hang here)",
                )
                .into()),
            };
            self.sched.tasks[me].wait = Wait::Ready;
            r?;
        }
    }

    /// A synchronisation point: with a non-zero schedule seed the scheduler may run another task first.
    pub(crate) fn preempt(&mut self) -> R<()> {
        if self.sched.seed == 0 || self.sched.tasks.len() < 2 {
            return Ok(());
        }
        let me = self.sched.current;
        if self.sched.next_rand() % 2 == 0 {
            return Ok(());
        }
        let cands = self.sched.runnable(me);
        if cands.is_empty() {
            return Ok(());
        }
        let t = cands[self.sched.next_rand() as usize % cands.len()];
        self.sched.tasks[me].wait = Wait::Ready;
        self.switch_to(t)
    }

    /// `thread::yield_now()`: let every other runnable task go first (FIFO), then continue.
    pub(crate) fn yield_now(&mut self) -> R<()> {
        let me = self.sched.current;
        let cands = self.sched.runnable(me);
        if let Some(&t) = cands.first() {
            self.sched.tasks[me].wait = Wait::Ready;
            self.switch_to(t)?;
        }
        Ok(())
    }

    pub(crate) fn sleep_ns(&mut self, ns: u64) -> R<()> {
        let until = self.sched.clock.saturating_add(ns);
        self.block_on(Wait::Sleep(until))
    }

    /// Start a new task running `f()`; it first runs when the spawner blocks (or yields).
    pub(crate) fn spawn_task(&mut self, f: Value, name: Option<String>) -> R<usize> {
        self.preempt()?;
        let id = self.sched.tasks.len();
        self.sched.tasks.push(Task { name, done: false, wait: Wait::Ready, ctx: Some(Ctx::fresh()), result: None });
        let baton = self.sched.baton.clone();
        let ptr = SendPtr(self as *mut Interpreter);
        let fv = SendVal(f);
        let spawned = std::thread::Builder::new().stack_size(TASK_STACK).spawn(move || {
            let (ptr, fv) = (ptr, fv);
            baton.wait_for(id);
            // SAFETY: we hold the baton.
            let it = unsafe { &mut *ptr.0 };
            let here = 0u8;
            it.stack_guard = Some((&here as *const u8 as usize, TASK_STACK - TASK_STACK_HEADROOM));
            let r = it.call_value(&fv.0, vec![]);
            drop(fv);
            it.finish_task(id, r);
        });
        if let Err(e) = spawned {
            return Err(RuntimeError::new(ErrorKind::Unsupported, format!("cannot start a thread: {e}")).into());
        }
        Ok(id)
    }

    /// Record the outcome of task `id` (running on its own thread) and pass the baton on.
    fn finish_task(&mut self, id: usize, r: R) {
        let outcome = match r {
            Ok(v) => Ok(v),
            Err(super::error::Ctl::Return(v)) => Ok(v),
            Err(super::error::Ctl::Err(e)) if e.kind == ErrorKind::Panic => {
                let name = self.sched.tasks[id].name.clone().unwrap_or_else(|| "<unnamed>".into());
                let text = self.annotate(*e.clone()).render().replacen("thread 'main'", &format!("thread '{}'", name), 1);
                self.write_err(&format!("{}\n", text.trim_end()));
                Err(e.message.clone())
            }
            Err(super::error::Ctl::Err(e)) => {
                self.sched.fatal = Some(self.annotate(*e));
                Err("fatal".to_string())
            }
            Err(_) => Ok(Value::Unit),
        };
        self.sched.tasks[id].result = Some(outcome);
        self.sched.tasks[id].done = true;
        let next = if self.sched.fatal.is_some() { Some(0) } else { self.sched.pick(id, false) };
        let target = match next {
            Some(t) => t,
            None => {
                self.sched.fatal = Some(RuntimeError::new(ErrorKind::Semantic, "deadlock: every thread is blocked"));
                0
            }
        };
        let theirs = self.sched.tasks[target].ctx.take().unwrap_or_else(Ctx::fresh);
        let _discard = self.take_ctx();
        self.put_ctx(theirs);
        self.sched.current = target;
        let baton = self.sched.baton.clone();
        baton.set(target);
    }

    pub(crate) fn task_result(&mut self, id: usize) -> Result<Value, String> {
        match self.sched.tasks[id].result.take() {
            Some(r) => r,
            None => Err("thread result already taken".into()),
        }
    }
}
