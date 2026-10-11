//! `std::sync::mpsc` channels over the cooperative scheduler.
//!
//! Sender/receiver liveness is tracked with tokens: a `Sender` value (and every copy of it the
//! interpreter makes) shares one token, `tx.clone()` mints a new one, and a token whose last copy is
//! gone no longer counts. That reproduces "the channel closes when every sender is dropped" for
//! scope exit and for closures finishing, without explicit drop bookkeeping.

use super::*;
use crate::interpreter::iter::IterSrc;
use crate::interpreter::native::{downcast, Native};
use crate::interpreter::sched::Wait;
use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Weak;

pub struct EndToken;

pub struct Chan {
    q: RefCell<VecDeque<Value>>,
    cap: Option<usize>,
    senders: RefCell<Vec<Weak<EndToken>>>,
    receiver: RefCell<Weak<EndToken>>,
}

impl Chan {
    pub fn has_message(&self) -> bool {
        !self.q.borrow().is_empty()
    }
    pub fn has_room(&self) -> bool {
        match self.cap {
            None => true,
            Some(c) => self.q.borrow().len() < c.max(1),
        }
    }
    pub fn senders_alive(&self) -> bool {
        let mut s = self.senders.borrow_mut();
        s.retain(|w| w.strong_count() > 0);
        !s.is_empty()
    }
    pub fn receiver_alive(&self) -> bool {
        self.receiver.borrow().strong_count() > 0
    }
}

struct SenderObj {
    chan: Rc<Chan>,
    _token: Rc<EndToken>,
}
struct ReceiverObj {
    chan: Rc<Chan>,
    _token: Rc<EndToken>,
}
/// Error values with a fixed Debug/Display text.
struct ErrObj(&'static str, &'static str);

macro_rules! native_impl {
    ($t:ty, $name:literal) => {
        impl Native for $t {
            fn type_name(&self) -> &'static str {
                $name
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn for_each_value(&self, f: &mut dyn FnMut(&Value)) {
                for v in self.chan.q.borrow().iter() {
                    f(v);
                }
            }
        }
    };
}
native_impl!(SenderObj, "Sender");
native_impl!(ReceiverObj, "Receiver");

impl Native for ErrObj {
    fn type_name(&self) -> &'static str {
        self.0
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn debug(&self) -> String {
        self.1.to_string()
    }
    fn display(&self) -> Option<String> {
        match self.0 {
            "RecvError" => Some("receiving on a closed channel".into()),
            "SendError" => Some("sending on a closed channel".into()),
            _ => None,
        }
    }
}

fn err_obj(name: &'static str, debug: &'static str) -> Value {
    Value::Native(Rc::new(ErrObj(name, debug)))
}

fn new_sender(chan: &Rc<Chan>) -> Value {
    let token = Rc::new(EndToken);
    chan.senders.borrow_mut().push(Rc::downgrade(&token));
    Value::Native(Rc::new(SenderObj { chan: chan.clone(), _token: token }))
}

fn make_channel(cap: Option<usize>) -> Value {
    let rx_token = Rc::new(EndToken);
    let chan = Rc::new(Chan {
        q: RefCell::new(VecDeque::new()),
        cap,
        senders: RefCell::new(Vec::new()),
        receiver: RefCell::new(Rc::downgrade(&rx_token)),
    });
    let tx = new_sender(&chan);
    let rx = Value::Native(Rc::new(ReceiverObj { chan, _token: rx_token }));
    Value::tuple(vec![tx, rx])
}

fn chan_of(v: &Value) -> R<(Rc<Chan>, bool)> {
    if let Some(s) = downcast::<SenderObj>(v) {
        return Ok((s.chan.clone(), true));
    }
    if let Some(r) = downcast::<ReceiverObj>(v) {
        return Ok((r.chan.clone(), false));
    }
    Err(semantic("expected a channel end"))
}

impl Interpreter {
    /// One blocking `recv` (None = disconnected) or a non-blocking attempt.
    pub(crate) fn chan_recv_item(&mut self, chan: &Rc<Chan>, try_only: bool) -> R<Option<Value>> {
        if !try_only {
            self.preempt()?;
            self.block_on(Wait::Recv(chan.clone(), None))?;
        }
        Ok(chan.q.borrow_mut().pop_front())
    }
}

pub(crate) fn receiver_iter(v: &Value, try_only: bool) -> Option<Value> {
    let r = downcast::<ReceiverObj>(v)?;
    Some(Value::Iter(IterState::new(IterSrc::Recv(r.chan.clone(), try_only))))
}

fn send(it: &mut Interpreter, tx: &Value, a: &[Value], blocking: bool) -> R {
    let (chan, _) = chan_of(tx)?;
    let v = raw_arg(a, 0, "send")?;
    it.preempt()?;
    if !chan.receiver_alive() {
        return Ok(Value::err(err_obj("SendError", "SendError { .. }")));
    }
    match chan.cap {
        Some(c) if !chan.has_room() => {
            if !blocking {
                return Ok(Value::err(err_obj("TrySendError", "Full(..)")));
            }
            it.block_on(Wait::Room(chan.clone()))?;
            if !chan.receiver_alive() {
                return Ok(Value::err(err_obj("SendError", "SendError { .. }")));
            }
            chan.q.borrow_mut().push_back(v);
            let _ = c;
        }
        Some(0) => {
            chan.q.borrow_mut().push_back(v);
            if blocking {
                it.block_on(Wait::Drained(chan.clone()))?;
            }
        }
        _ => chan.q.borrow_mut().push_back(v),
    }
    Ok(Value::ok(Value::Unit))
}

/// The argument with references followed (like `arg`) but handles/natives left intact.
pub(crate) fn raw_arg(args: &[Value], i: usize, what: &str) -> R<Value> {
    let v = args.get(i).cloned().ok_or_else(|| semantic(format!("`{}` is missing argument {}", what, i + 1)))?;
    crate::interpreter::place::strip_refs(v)
}

pub fn register(r: &mut Registry) {
    r.func("mpsc::channel", |_, _| Ok(make_channel(None)));
    r.func("mpsc::sync_channel", |_, a| Ok(make_channel(Some(arg_usize(&a, 0, "sync_channel")?))));
    r.native("Sender", "send", |it, v, a| send(it, v, &a, true));
    r.native("Sender", "try_send", |it, v, a| send(it, v, &a, false));
    r.native("Sender", "clone", |_, v, _| Ok(new_sender(&chan_of(v)?.0)));
    r.native("Receiver", "recv", |it, v, _| {
        let (chan, _) = chan_of(v)?;
        Ok(match it.chan_recv_item(&chan, false)? {
            Some(x) => Value::ok(x),
            None => Value::err(err_obj("RecvError", "RecvError")),
        })
    });
    r.native("Receiver", "try_recv", |it, v, _| {
        let (chan, _) = chan_of(v)?;
        Ok(match it.chan_recv_item(&chan, true)? {
            Some(x) => Value::ok(x),
            None if chan.senders_alive() => Value::err(Value::enum_val("TryRecvError", "Empty", 0, Payload::Unit)),
            None => Value::err(Value::enum_val("TryRecvError", "Disconnected", 1, Payload::Unit)),
        })
    });
    r.native("Receiver", "recv_timeout", |it, v, a| {
        let (chan, _) = chan_of(v)?;
        let ns = super::time::duration_ns(&arg(&a, 0, "recv_timeout")?)?;
        it.preempt()?;
        let deadline = it.sched.clock.saturating_add(ns as u64);
        it.block_on(Wait::Recv(chan.clone(), Some(deadline)))?;
        let msg = chan.q.borrow_mut().pop_front();
        Ok(match msg {
            Some(x) => Value::ok(x),
            None if chan.senders_alive() => Value::err(Value::enum_val("RecvTimeoutError", "Timeout", 0, Payload::Unit)),
            None => Value::err(Value::enum_val("RecvTimeoutError", "Disconnected", 1, Payload::Unit)),
        })
    });
    for n in ["iter", "into_iter"] {
        r.native("Receiver", n, |_, v, _| receiver_iter(v, false).ok_or_else(|| semantic("not a receiver")));
    }
    r.native("Receiver", "try_iter", |_, v, _| receiver_iter(v, true).ok_or_else(|| semantic("not a receiver")));
}
