//! The body of a `Value::Vec`: a `Vec<Value>`, or, for a vector whose elements are all scalars of one
//! type (`bool`, an integer width that fits `i64`, `f64`), the scalars packed (1 or 8 bytes each instead
//! of a 32-byte `Value`).
//!
//! Every reader that wants `&[Value]` gets one (`Deref`): a packed body unpacks into a cache the first
//! time and keeps it until the next write; every writer that wants `&mut Vec<Value>` (`DerefMut`) unpacks
//! the body for good. The hot paths (`len`, element reads and stores, `push`, the machine-code loop
//! helpers) work on the packed form directly through the inherent methods below, so a packed vector
//! answers every question exactly as the unpacked one does.

use super::int::IntTy;
use super::value::{FloatTy, Value};
use std::cell::OnceCell;

#[derive(Clone, Debug)]
pub enum Packed {
    Bool(Vec<bool>),
    /// Values of width `t` (a width whose values fit `i64`: not `u64`/`usize`/`i128`/`u128`).
    Int(Vec<i64>, IntTy),
    Float(Vec<f64>, FloatTy),
}

#[derive(Clone, Debug)]
pub enum VecBody {
    Values(Vec<Value>),
    Packed(Packed, OnceCell<Vec<Value>>),
}

fn int_packable(t: IntTy) -> bool {
    !matches!(t, IntTy::U64 | IntTy::Usize | IntTy::I128 | IntTy::U128)
}

impl Packed {
    fn len(&self) -> usize {
        match self {
            Packed::Bool(v) => v.len(),
            Packed::Int(v, _) => v.len(),
            Packed::Float(v, _) => v.len(),
        }
    }

    fn get(&self, i: usize) -> Option<Value> {
        Some(match self {
            Packed::Bool(v) => Value::Bool(*v.get(i)?),
            Packed::Int(v, t) => Value::Int(*v.get(i)? as i128, *t),
            Packed::Float(v, t) => Value::Float(*v.get(i)?, *t),
        })
    }

    fn unpack(&self) -> Vec<Value> {
        match self {
            Packed::Bool(v) => v.iter().map(|b| Value::Bool(*b)).collect(),
            Packed::Int(v, t) => v.iter().map(|x| Value::Int(*x as i128, *t)).collect(),
            Packed::Float(v, t) => v.iter().map(|x| Value::Float(*x, *t)).collect(),
        }
    }

    /// An empty packing for `x`'s type, if it has one.
    fn for_value(x: &Value) -> Option<Packed> {
        Some(match x {
            Value::Bool(_) => Packed::Bool(Vec::new()),
            Value::Int(i, t) if int_packable(*t) && i64::try_from(*i).is_ok() => Packed::Int(Vec::new(), *t),
            Value::Float(_, t) => Packed::Float(Vec::new(), *t),
            _ => return None,
        })
    }

    /// Store `x` at `i` (or append it when `i == len`) if it has this packing's exact type.
    fn put(&mut self, i: usize, x: &Value) -> bool {
        fn at<T>(v: &mut Vec<T>, i: usize, x: T) -> bool {
            if i < v.len() {
                v[i] = x;
            } else {
                v.push(x);
            }
            true
        }
        match (self, x) {
            (Packed::Bool(v), Value::Bool(b)) => at(v, i, *b),
            (Packed::Int(v, t), Value::Int(n, u)) if t == u => match i64::try_from(*n) {
                Ok(n) => at(v, i, n),
                Err(_) => false,
            },
            (Packed::Float(v, t), Value::Float(f, u)) if t == u => at(v, i, *f),
            _ => false,
        }
    }
}

impl Default for VecBody {
    fn default() -> VecBody {
        VecBody::Values(Vec::new())
    }
}

impl From<Vec<Value>> for VecBody {
    fn from(v: Vec<Value>) -> VecBody {
        VecBody::Values(v)
    }
}

impl std::ops::Deref for VecBody {
    type Target = Vec<Value>;
    fn deref(&self) -> &Vec<Value> {
        match self {
            VecBody::Values(v) => v,
            VecBody::Packed(p, cache) => cache.get_or_init(|| p.unpack()),
        }
    }
}

impl std::ops::DerefMut for VecBody {
    fn deref_mut(&mut self) -> &mut Vec<Value> {
        if let VecBody::Packed(p, cache) = self {
            let vals = cache.take().unwrap_or_else(|| p.unpack());
            *self = VecBody::Values(vals);
        }
        match self {
            VecBody::Values(v) => v,
            VecBody::Packed(..) => unreachable!("unpacked above"),
        }
    }
}

impl VecBody {
    /// The number of elements (no unpacking).
    pub fn len(&self) -> usize {
        match self {
            VecBody::Values(v) => v.len(),
            VecBody::Packed(p, _) => p.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Element `i`, by value (no unpacking).
    pub fn get_value(&self, i: usize) -> Option<Value> {
        match self {
            VecBody::Values(v) => v.get(i).cloned(),
            VecBody::Packed(p, _) => p.get(i),
        }
    }

    /// The packed scalars, if the body is packed.
    pub fn packed(&self) -> Option<&Packed> {
        match self {
            VecBody::Packed(p, _) => Some(p),
            VecBody::Values(_) => None,
        }
    }

    /// `self[i] = x` as a plain replacement (`i < len`); a value the packing cannot hold unpacks first.
    pub fn set_value(&mut self, i: usize, x: Value) {
        if let VecBody::Packed(p, cache) = self {
            if p.put(i, &x) {
                cache.take();
                return;
            }
        }
        (**self)[i] = x;
    }

    /// `push`: an empty vector starts packed when `x` is a packable scalar; a packed one stays packed
    /// while the elements keep its type.
    pub fn push_value(&mut self, x: Value) {
        match self {
            VecBody::Values(v) if v.is_empty() => {
                if let Some(mut p) = Packed::for_value(&x) {
                    p.put(0, &x);
                    *self = VecBody::Packed(p, OnceCell::new());
                    return;
                }
                v.push(x);
            }
            VecBody::Packed(p, cache) => {
                let n = p.len();
                if p.put(n, &x) {
                    cache.take();
                    return;
                }
                (**self).push(x);
            }
            VecBody::Values(v) => v.push(x),
        }
    }
}
