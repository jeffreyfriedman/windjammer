//! `std::thread`: spawn/join/scope/sleep over the cooperative scheduler (`sched.rs`).

use super::channel::raw_arg;
use super::*;
use crate::interpreter::heap::Walker;
use crate::interpreter::native::{downcast, Native};
use crate::interpreter::sched::Wait;
use std::any::Any;

struct JoinHandleObj(usize);
/// ADR-0004 `Task<T>` (`spawn f(x)`, `async f(x)`): `wait()` returns the value, a panic propagates.
struct TaskObj(usize);
struct ThreadObj(usize, Option<String>);
struct ScopeObj(RefCell<Vec<usize>>);
struct BuilderObj(RefCell<Option<String>>);
/// The `Box<dyn Any + Send>` a panicking thread's `join()` returns.
struct PanicPayload(String);

macro_rules! plain {
    ($t:ty, $name:literal) => {
        impl Native for $t {
            fn type_name(&self) -> &'static str {
                $name
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}
plain!(JoinHandleObj, "JoinHandle");
plain!(TaskObj, "Task");
plain!(ScopeObj, "Scope");
plain!(BuilderObj, "Builder");
impl Native for ThreadObj {
    fn type_name(&self) -> &'static str {
        "Thread"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
impl Native for PanicPayload {
    fn type_name(&self) -> &'static str {
        "Any"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn debug(&self) -> String {
        "Any { .. }".into()
    }
}

fn is_resource(v: &Value) -> bool {
    let mut found = false;
    Walker::new().walk(v, &mut |x| match x {
        Value::Native(n) if n.order_key().is_none() => found = true,
        Value::Rc(_) | Value::Cell(..) => found = true,
        Value::Struct(s) if s.uid != 0 => found = true,
        Value::Ref(p) if p.guard.is_some() => found = true,
        _ => {}
    });
    found
}

/// `move` semantics for a thread closure: captured variables the body mentions are copied into the
/// closure; those holding resources (channel ends, `Arc`s, guards, values with destructors) are
/// moved out of the spawning scope, so their lifetime is the thread's.
pub(crate) fn move_captures(f: Value) -> Value {
    let Value::Func(c) = &f else { return f };
    let Callable::Closure { params, body, env, module, self_ty, tparams } = &**c else { return f };
    let text = crate::source_map::with_elided_paths(|| format!("{:?}", body));
    let mut new_env = Vec::with_capacity(env.len());
    for (name, slot) in env {
        if !text.contains(&format!("name: {:?}", name)) {
            new_env.push((*name, slot.clone()));
            continue;
        }
        let v = slot.borrow().clone();
        if is_resource(&v) && !matches!(v, Value::Unit) {
            *slot.borrow_mut() = Value::Unit;
        }
        new_env.push((*name, Rc::new(RefCell::new(v))));
    }
    Value::Func(Rc::new(Callable::Closure {
        params,
        body,
        env: new_env,
        module: *module,
        self_ty: self_ty.clone(),
        tparams: tparams.clone(),
    }))
}

fn handle(id: usize) -> Value {
    Value::Native(Rc::new(JoinHandleObj(id)))
}

fn join(it: &mut Interpreter, id: usize) -> R {
    it.preempt()?;
    it.block_on(Wait::Join(id))?;
    Ok(match it.task_result(id) {
        Ok(v) => Value::ok(v),
        Err(msg) => Value::err(Value::Native(Rc::new(PanicPayload(msg)))),
    })
}

/// Start `f()` as an ADR-0004 task; it joins the innermost `scope { }` of the spawning task if there is one.
pub(crate) fn start_task(it: &mut Interpreter, f: Value) -> R {
    let me = it.sched.current;
    let id = it.spawn_task(f, None)?;
    if let Some(top) = it.sched.tasks[me].scopes.last() {
        top.borrow_mut().push(id);
    }
    Ok(Value::Native(Rc::new(TaskObj(id))))
}

/// `scope { body }` (ADR-0004): evaluate the block in place (so `return`, `?` and `break` leave it as from any block),
/// then wait for every task started inside it, also by those tasks and also when the block ended early. The first
/// panic of a task propagates after all of them have finished; an error of the block itself wins.
pub(crate) fn run_scope(it: &mut Interpreter, body: &'static crate::parser::Expression<'static>, at: Option<&'static crate::source_map::Location>) -> R {
    let list: crate::interpreter::sched::TaskList = Rc::new(RefCell::new(Vec::new()));
    let me = it.sched.current;
    it.sched.tasks[me].scopes.push(list.clone());
    let out = it.eval(body);
    it.sched.tasks[me].scopes.pop();
    let mut first_panic = None;
    let mut i = 0;
    loop {
        let next = list.borrow().get(i).copied();
        let Some(id) = next else { break };
        i += 1;
        it.block_on(Wait::Join(id))?;
        if let Some(Err(msg)) = &it.sched.tasks[id].result {
            first_panic.get_or_insert_with(|| msg.clone());
        }
    }
    let v = out?;
    match first_panic {
        Some(msg) => {
            it.mark(at); // the panic leaves the program at the scope
            Err(panic_ctl(msg))
        }
        None => Ok(v),
    }
}

fn wait_task(it: &mut Interpreter, id: usize) -> R {
    it.preempt()?;
    it.block_on(Wait::Join(id))?;
    match it.sched.tasks[id].result.clone() {
        Some(Ok(v)) => Ok(v),
        Some(Err(msg)) => Err(panic_ctl(msg)),
        None => Err(semantic("task result missing")),
    }
}

/// `.wait()` / `.await` on a task value; `None` when `v` is not a task.
pub(crate) fn wait_if_task(it: &mut Interpreter, v: &Value) -> Option<R> {
    downcast::<TaskObj>(v).map(|t| t.0).map(|id| wait_task(it, id))
}

pub fn register(r: &mut Registry) {
    r.type_name("Task");
    r.native("Task", "wait", |it, v, _| wait_task(it, downcast::<TaskObj>(v).map(|t| t.0).ok_or_else(|| semantic("not a Task"))?));
    r.native("Task", "is_finished", |it, v, _| {
        let id = downcast::<TaskObj>(v).map(|t| t.0).ok_or_else(|| semantic("not a Task"))?;
        Ok(Value::Bool(it.sched.tasks[id].done))
    });
    for t in ["thread", "JoinHandle"] {
        r.type_name(t);
    }
    r.func("thread::spawn", |it, a| {
        let f = move_captures(raw_arg(&a, 0, "spawn")?);
        Ok(handle(it.spawn_task(f, None)?))
    });
    r.func("thread::sleep", |it, a| {
        it.sleep_ns(super::time::duration_ns(&arg(&a, 0, "sleep")?)?.max(0) as u64)?;
        Ok(Value::Unit)
    });
    r.func("thread::yield_now", |it, _| {
        it.yield_now()?;
        Ok(Value::Unit)
    });
    r.func("thread::current", |it, _| {
        let id = it.sched.current;
        Ok(Value::Native(Rc::new(ThreadObj(id, it.sched.tasks[id].name.clone()))))
    });
    r.func("thread::panicking", |_, _| Ok(Value::Bool(false)));
    r.func("thread::scope", |it, a| {
        let f = arg(&a, 0, "scope")?;
        let scope = Value::Native(Rc::new(ScopeObj(RefCell::new(Vec::new()))));
        let out = it.call_value(&f, vec![scope.clone()]);
        let ids = downcast::<ScopeObj>(&scope).map(|s| s.0.borrow().clone()).unwrap_or_default();
        let mut panicked = false;
        for id in ids {
            it.block_on(Wait::Join(id))?;
            if it.sched.tasks[id].result.as_ref().map_or(false, |r| r.is_err()) {
                panicked = true;
            }
        }
        let v = out?;
        if panicked {
            return Err(panic_ctl("a scoped thread panicked"));
        }
        Ok(v)
    });
    r.native("Scope", "spawn", |it, v, a| {
        // scoped threads borrow their environment: no capture move
        let f = raw_arg(&a, 0, "spawn")?;
        let id = it.spawn_task(f, None)?;
        if let Some(s) = downcast::<ScopeObj>(v) {
            s.0.borrow_mut().push(id);
        }
        Ok(handle(id))
    });
    r.func("Builder::new", |_, _| Ok(Value::Native(Rc::new(BuilderObj(RefCell::new(None))))));
    r.func("thread::Builder::new", |_, _| Ok(Value::Native(Rc::new(BuilderObj(RefCell::new(None))))));
    r.native("Builder", "name", |_, v, a| {
        if let Some(b) = downcast::<BuilderObj>(v) {
            *b.0.borrow_mut() = Some(arg_str(&a, 0, "name")?.to_string());
        }
        Ok(v.clone())
    });
    r.native("Builder", "stack_size", |_, v, _| Ok(v.clone()));
    r.native("Builder", "spawn", |it, v, a| {
        let name = downcast::<BuilderObj>(v).and_then(|b| b.0.borrow().clone());
        let f = move_captures(raw_arg(&a, 0, "spawn")?);
        Ok(Value::ok(handle(it.spawn_task(f, name)?)))
    });
    r.native("JoinHandle", "join", |it, v, _| {
        let id = downcast::<JoinHandleObj>(v).map(|h| h.0).ok_or_else(|| semantic("not a JoinHandle"))?;
        join(it, id)
    });
    r.native("JoinHandle", "is_finished", |it, v, _| {
        let id = downcast::<JoinHandleObj>(v).map(|h| h.0).ok_or_else(|| semantic("not a JoinHandle"))?;
        Ok(Value::Bool(it.sched.tasks[id].done))
    });
    r.native("JoinHandle", "thread", |it, v, _| {
        let id = downcast::<JoinHandleObj>(v).map(|h| h.0).ok_or_else(|| semantic("not a JoinHandle"))?;
        Ok(Value::Native(Rc::new(ThreadObj(id, it.sched.tasks[id].name.clone()))))
    });
    r.native("Thread", "id", |_, v, _| {
        let id = downcast::<ThreadObj>(v).map(|t| t.0).unwrap_or(0);
        Ok(Value::Native(Rc::new(ThreadIdObj(id as i128 + 1))))
    });
    r.native("Thread", "name", |_, v, _| {
        Ok(Value::option(downcast::<ThreadObj>(v).and_then(|t| t.1.clone()).map(Value::str)))
    });
    r.native("Thread", "unpark", |_, _, _| Ok(Value::Unit));
    r.native("Any", "downcast_ref", |_, v, _| {
        Ok(Value::some(Value::str(downcast::<PanicPayload>(v).map(|p| p.0.clone()).unwrap_or_default())))
    });
}

struct ThreadIdObj(i128);
impl Native for ThreadIdObj {
    fn type_name(&self) -> &'static str {
        "ThreadId"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn order_key(&self) -> Option<i128> {
        Some(self.0)
    }
    fn debug(&self) -> String {
        format!("ThreadId({})", self.0)
    }
}

/// `select { }` (ADR-0004, docs/concurrency/PLAN.md stage 2a): evaluate the arms' sources once, then run the first
/// ready arm in source order (also under a schedule seed: the choice is part of the semantics, SPEC-CONC-010), or
/// `else` when none is ready; wait
/// on all of them otherwise. A closed, empty channel is never ready; when nothing can become ready the select panics.
pub(crate) fn run_select(it: &mut Interpreter, arms: &'static [crate::parser::SelectArm<'static>]) -> R {
    use crate::parser::SelectKind as K;
    enum Src {
        Chan(Rc<super::channel::Chan>),
        Task(usize),
        Until(u64),
        Else,
    }
    let mut srcs = Vec::with_capacity(arms.len());
    for a in arms {
        srcs.push(match (a.kind, a.source) {
            (K::Recv, Some(e)) => {
                let v = it.eval(e)?;
                Src::Chan(super::channel::chan_of(&v)?.0)
            }
            (K::Wait, Some(e)) => {
                let v = it.eval(e)?;
                Src::Task(downcast::<TaskObj>(&v).map(|t| t.0).ok_or_else(|| semantic("`select`: `.wait()` on a value that is not a Task"))?)
            }
            (K::Timeout, Some(e)) => {
                let d = it.eval(e)?;
                let ns = super::time::duration_ns(&d)?.max(0) as u64;
                Src::Until(it.sched.clock.saturating_add(ns))
            }
            _ => Src::Else,
        });
    }
    it.preempt()?;
    loop {
        let ready: Vec<usize> = srcs
            .iter()
            .enumerate()
            .filter(|(_, s)| match s {
                Src::Chan(c) => c.has_message(),
                Src::Task(t) => it.sched.tasks[*t].done,
                Src::Until(u) => it.sched.clock >= *u,
                Src::Else => false,
            })
            .map(|(i, _)| i)
            .collect();
        if let Some(&first) = ready.first() {
            let i = first;
            let v = match &srcs[i] {
                Src::Chan(c) => it.chan_recv_item(c, true)?.unwrap_or(Value::Unit),
                Src::Task(t) => wait_task(it, *t)?,
                _ => Value::Unit,
            };
            return select_arm(it, &arms[i], v);
        }
        if let Some(i) = srcs.iter().position(|s| matches!(s, Src::Else)) {
            return select_arm(it, &arms[i], Value::Unit);
        }
        let waits: Vec<Wait> = srcs
            .iter()
            .filter_map(|s| match s {
                Src::Chan(c) if c.senders_alive() => Some(Wait::Recv(c.clone(), None)),
                Src::Task(t) => Some(Wait::Join(*t)),
                Src::Until(u) => Some(Wait::Sleep(*u)),
                _ => None,
            })
            .collect();
        if waits.is_empty() {
            return Err(panic_ctl("every channel of this `select` is closed"));
        }
        it.block_on(Wait::Any(waits))?;
    }
}

fn select_arm(it: &mut Interpreter, arm: &'static crate::parser::SelectArm<'static>, v: Value) -> R {
    it.push_scope();
    let mut binds = Vec::new();
    if !it.match_pattern(&arm.pattern, &v, None, false, &mut binds) {
        return it.pop_scope(Err(semantic("`select`: the arm's pattern does not match the value")));
    }
    for (n, b) in binds {
        it.define(n, b);
    }
    let r = it.eval(arm.body);
    it.pop_scope(r)
}
