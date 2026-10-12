//! Host-side objects exposed to programs as opaque values (`JoinHandle`, `Sender`, `Duration`, ...).
//!
//! A native object is an `Rc<dyn Native>`: cloning the value shares the object (so a `Sender`
//! stays one logical sender until every copy of the value is gone). Methods are registered per
//! `type_name` in `builtins` (`Registry::native`).

use super::value::Value;
use std::any::Any;

impl std::fmt::Debug for dyn Native {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.debug())
    }
}

pub trait Native: Any {
    fn type_name(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
    /// Values this object keeps alive (queued messages, results): roots for reachability.
    fn for_each_value(&self, _f: &mut dyn FnMut(&Value)) {}
    /// A total order key for value-like objects (`Duration`, `Instant`): enables `==`, `<`, sorting.
    fn order_key(&self) -> Option<i128> {
        None
    }
    /// `{}` text, for objects that implement `Display` (error values, `Duration` has none).
    fn display(&self) -> Option<String> {
        None
    }
    /// `{:?}` text.
    fn debug(&self) -> String {
        format!("{} {{ .. }}", self.type_name())
    }
}

pub fn downcast<'a, T: Native>(v: &'a Value) -> Option<&'a T> {
    match v {
        Value::Native(n) => n.as_any().downcast_ref::<T>(),
        _ => None,
    }
}
