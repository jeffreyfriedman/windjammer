//! `std::arena` (ADR-0016): `Arena<T>`, `Handle<T>`, `Scratch` on the interpreter heap.
//!
//! Same observable behaviour as `windjammer_runtime::arena` in a debug build: handles are checked on every
//! access (stale handle after `reset()` and foreign handle both panic with the runtime's messages). Scratch
//! values are ordinary heap strings/lists (the Go/JS story); the compile-time escape check
//! (`analyzer::arena_escape`) runs on interpreted programs too, so a program the compiler rejects never runs.

use super::*;
use crate::interpreter::error::Ctl;

const ARENA: &str = "__Arena";
const HANDLE: &str = "__Handle";
const SCRATCH: &str = "__Scratch";

fn strukt(name: &str, copy: bool, fields: Vec<(&str, Value)>) -> Value {
    Value::Struct(Rc::new(StructVal {
        name: Rc::from(name),
        fields: fields.into_iter().map(|(n, v)| (Rc::from(n), v)).collect(),
        copy,
        uid: 0,
    }))
}

fn field(v: &Value, name: &str) -> Value {
    match v {
        Value::Struct(s) => s.fields.iter().find(|(n, _)| &**n == name).map(|(_, v)| v.clone()).unwrap_or(Value::Unit),
        _ => Value::Unit,
    }
}

fn int(v: &Value) -> i128 {
    match v {
        Value::Int(n, _) => *n,
        _ => -1,
    }
}

fn is(v: &Value, name: &str) -> bool {
    matches!(v, Value::Struct(s) if &*s.name == name)
}

fn set_field(v: &mut Value, name: &str, x: Value) {
    if let Value::Struct(s) = v {
        let s = Rc::make_mut(s);
        if let Some(slot) = s.fields.iter_mut().find(|(n, _)| &**n == name) {
            slot.1 = x;
        }
    }
}

fn items_mut(v: &mut Value) -> &mut Vec<Value> {
    let Value::Struct(s) = v else { unreachable!("checked by caller") };
    let s = Rc::make_mut(s);
    let slot = s.fields.iter_mut().find(|(n, _)| &**n == "items").expect("arena items");
    if !matches!(slot.1, Value::Vec(_)) {
        slot.1 = Value::vec(Vec::new());
    }
    match &mut slot.1 {
        Value::Vec(x) => Rc::make_mut(x),
        _ => unreachable!(),
    }
}

fn new_arena(it: &mut Interpreter) -> Value {
    it.next_uid += 1;
    strukt(
        ARENA,
        false,
        vec![
            ("items", Value::vec(Vec::new())),
            ("generation", Value::typed(0, IntTy::U32)),
            ("id", Value::typed(it.next_uid as i128, IntTy::U32)),
        ],
    )
}

/// Index of `h` in `arena`, after the debug-build checks of the Rust runtime.
fn checked_index(arena: &Value, h: &Value) -> R<usize> {
    if !is(h, HANDLE) {
        return Err(semantic(format!("arena access expects a Handle, found {}", h.type_name())));
    }
    if int(&field(h, "arena_id")) != int(&field(arena, "id")) {
        return Err(panic_ctl("arena handle used with a different arena"));
    }
    let (hg, ag) = (int(&field(h, "generation")), int(&field(arena, "generation")));
    if hg != ag {
        return Err(panic_ctl(format!(
            "stale arena handle: allocated before reset (generation {hg}, arena is at generation {ag})"
        )));
    }
    Ok(int(&field(h, "index")) as usize)
}

fn not_arena(method: &str, v: &Value) -> Ctl {
    semantic(format!("no method named `{}` found for value of type `{}`", method, v.type_name()))
}

pub(super) fn register(r: &mut Registry) {
    for t in ["Arena", "Handle", "Scratch"] {
        r.type_name(t);
    }
    for n in ["Arena::new", "Arena::with_capacity"] {
        r.func(n, |it, _| Ok(new_arena(it)));
    }
    for n in ["Scratch::new", "Scratch::with_capacity"] {
        r.func(n, |_, _| Ok(strukt(SCRATCH, false, vec![("generation", Value::typed(0, IntTy::U32))])));
    }

    r.mutating(Kind::Struct, "alloc", |_, v, a| {
        let x = arg(&a, 0, "alloc")?;
        if is(v, SCRATCH) {
            return Ok(x); // GC heap: the value simply lives as long as it is referenced
        }
        if !is(v, ARENA) {
            return Err(not_arena("alloc", v));
        }
        let (generation, id) = (field(v, "generation"), field(v, "id"));
        let items = items_mut(v);
        items.push(x);
        let index = Value::typed((items.len() - 1) as i128, IntTy::U32);
        Ok(strukt(HANDLE, true, vec![("index", index), ("generation", generation), ("arena_id", id)]))
    });
    r.mutating(Kind::Struct, "reset", |_, v, _| {
        if !(is(v, ARENA) || is(v, SCRATCH)) {
            return Err(not_arena("reset", v));
        }
        if is(v, ARENA) {
            items_mut(v).clear();
        }
        let g = int(&field(v, "generation"));
        set_field(v, "generation", Value::typed((g + 1) % (1 << 32), IntTy::U32));
        Ok(Value::Unit)
    });
    r.pure(Kind::Struct, "get", |_, v, a| {
        if !is(v, ARENA) {
            return Err(not_arena("get", v));
        }
        let h = arg(&a, 0, "get")?;
        let i = checked_index(v, &h)?;
        match field(v, "items") {
            Value::Vec(items) => items.get(i).cloned().ok_or_else(|| panic_ctl("arena handle out of bounds")),
            _ => Err(panic_ctl("arena handle out of bounds")),
        }
    });
    r.pure(Kind::Struct, "try_get", |_, v, a| {
        if !is(v, ARENA) {
            return Err(not_arena("try_get", v));
        }
        let h = arg(&a, 0, "try_get")?;
        Ok(match checked_index(v, &h) {
            Ok(i) => match field(v, "items") {
                Value::Vec(items) => items.get(i).cloned().map(Value::some).unwrap_or_else(Value::none),
                _ => Value::none(),
            },
            Err(_) => Value::none(),
        })
    });
    r.place(Kind::Struct, "get_mut", |_, p, a| {
        let v = p.read()?;
        if !is(&v, ARENA) {
            return Err(not_arena("get_mut", &v));
        }
        let h = arg(&a, 0, "get_mut")?;
        let i = checked_index(&v, &h)?;
        Ok(Value::Ref(p.proj(Proj::Field(Rc::from("items"))).proj(Proj::Index(i))))
    });
    r.place(Kind::Struct, "iter_mut", |_, p, _| {
        let v = p.read()?;
        if !is(&v, ARENA) {
            return Err(not_arena("iter_mut", &v));
        }
        let n = match field(&v, "items") {
            Value::Vec(x) => x.len(),
            _ => 0,
        };
        let items = p.proj(Proj::Field(Rc::from("items")));
        Ok(new_iter((0..n).map(|i| Value::Ref(items.proj(Proj::Index(i)))).collect()))
    });
    r.pure(Kind::Struct, "iter", |_, v, _| {
        if !is(v, ARENA) {
            return Err(not_arena("iter", v));
        }
        Ok(match field(v, "items") {
            Value::Vec(x) => new_iter(x.to_vec()),
            _ => new_iter(Vec::new()),
        })
    });
    r.pure(Kind::Struct, "len", |_, v, _| {
        if !is(v, ARENA) {
            return Err(not_arena("len", v));
        }
        Ok(Value::typed(match field(v, "items") {
            Value::Vec(x) => x.len() as i128,
            _ => 0,
        }, IntTy::I64))
    });
    r.pure(Kind::Struct, "is_empty", |_, v, _| {
        if !is(v, ARENA) {
            return Err(not_arena("is_empty", v));
        }
        Ok(Value::Bool(matches!(field(v, "items"), Value::Vec(x) if x.is_empty())))
    });
    r.pure(Kind::Struct, "generation", |_, v, _| {
        if !(is(v, ARENA) || is(v, SCRATCH)) {
            return Err(not_arena("generation", v));
        }
        Ok(Value::typed(int(&field(v, "generation")), IntTy::I64))
    });
    r.pure(Kind::Struct, "index", |_, v, _| {
        if !is(v, HANDLE) {
            return Err(not_arena("index", v));
        }
        Ok(Value::typed(int(&field(v, "index")), IntTy::I64))
    });
    r.pure(Kind::Struct, "str", |_, v, a| {
        if !is(v, SCRATCH) {
            return Err(not_arena("str", v));
        }
        Ok(Value::str(arg_str(&a, 0, "str")?.to_string()))
    });
    r.pure(Kind::Struct, "string", |_, v, _| {
        if !is(v, SCRATCH) {
            return Err(not_arena("string", v));
        }
        Ok(Value::str(String::new()))
    });
    r.pure(Kind::Struct, "list", |_, v, _| {
        if !is(v, SCRATCH) {
            return Err(not_arena("list", v));
        }
        Ok(Value::vec(Vec::new()))
    });
}
