//! `Default::default()` support.

use super::engine::Interpreter;
use super::error::{semantic, R};
use super::int::IntTy;
use super::value::*;
use crate::parser::Type;
use std::collections::BTreeMap;
use std::rc::Rc;

impl Interpreter {
    pub(crate) fn default_for(&mut self, t: &Type) -> R {
        if let Some(it) = IntTy::from_type(t) {
            return Ok(Value::Int(0, it));
        }
        if let Some(ft) = type_is_float(t) {
            return Ok(Value::Float(0.0, ft));
        }
        match t {
            Type::Bool => Ok(Value::Bool(false)),
            Type::String => Ok(Value::str("")),
            Type::Vec(_) | Type::Array(..) => Ok(Value::vec(vec![])),
            Type::Option(_) => Ok(Value::none()),
            Type::Tuple(ts) => {
                let mut out = Vec::new();
                for x in ts {
                    out.push(self.default_for(x)?);
                }
                Ok(Value::tuple(out))
            }
            Type::Reference(i) | Type::MutableReference(i) => self.default_for(i),
            Type::Parameterized(n, _) | Type::Custom(n) => {
                let base = n.rsplit("::").next().unwrap_or(n).split('<').next().unwrap_or(n).to_string();
                match base.as_str() {
                    "Vec" | "VecDeque" => return Ok(Value::vec(vec![])),
                    "HashMap" => return Ok(Value::hash_map(BTreeMap::new())),
                    "BTreeMap" => return Ok(Value::btree_map(BTreeMap::new())),
                    "HashSet" => return Ok(Value::hash_set(BTreeMap::new())),
                    "BTreeSet" => return Ok(Value::btree_set(BTreeMap::new())),
                    "String" | "str" | "string" => return Ok(Value::str("")),
                    "char" => return Ok(Value::Char('\0')),
                    "Option" => return Ok(Value::none()),
                    _ => {}
                }
                // a name shared by several modules: the written (or already resolved) path picks the type
                let base = if self.dup_types.is_empty() {
                    base
                } else if self.structs.contains_key(n.as_str()) {
                    n.clone()
                } else {
                    self.type_key(self.scope.module, n)
                };
                if let Some(f) = self.methods.get(&(base.clone(), "default".to_string())).cloned() {
                    return self.call_func(&f, vec![], None);
                }
                if let Some(def) = self.structs.get(&base) {
                    let fields: Vec<(&'static str, Type)> = def.fields.iter().map(|(n, t)| (*n, t.clone())).collect();
                    let copy = def.copy;
                    let mut out = Vec::new();
                    for (fname, ft) in fields {
                        out.push((Rc::from(fname), self.default_for(&ft)?));
                    }
                    return Ok(self.make_struct(base.as_str(), out, copy));
                }
                Err(semantic(format!("no default value for type `{}`", base)))
            }
            other => Err(semantic(format!("no default value for type {:?}", other))),
        }
    }

    /// The `Default` of a value's own type (for `mem::take`).
    pub(crate) fn default_like(&mut self, v: &Value) -> R {
        Ok(match v {
            Value::Int(_, t) => Value::Int(0, *t),
            Value::Float(_, t) => Value::Float(0.0, *t),
            Value::Bool(_) => Value::Bool(false),
            Value::Str(_) => Value::str(""),
            Value::Vec(_) => Value::vec(vec![]),
            Value::Map(m) => Value::Map(Rc::new(m.empty_like())),
            Value::Set(m) => Value::Set(Rc::new(m.empty_like())),
            Value::Char(_) => Value::Char('\0'),
            Value::Enum(e) if &*e.ty == "Option" => Value::none(),
            Value::Struct(s) => self.default_for(&Type::Custom(s.name.to_string()))?,
            Value::Unit => Value::Unit,
            other => return Err(semantic(format!("no default for {}", other.type_name()))),
        })
    }
}
