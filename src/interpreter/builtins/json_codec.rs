//! Typed JSON for interpreted programs (ADR-0027): the interpreter rides serde's data model instead of
//! imitating it. `Enc` implements `serde::Serialize` over a runtime value; `Dec` implements
//! `DeserializeSeed` driven by the declared type, calling the same `Deserializer` methods and the same
//! `de::Error` constructors that `#[derive(Deserialize)]` calls, and delegating scalars to serde's own
//! impls. Both engines then share `serde_json` and `serde_path_to_error`, so bytes, error texts, line,
//! column and JSONPath agree by construction (checked by `tests/json_conformance`).

use super::*;
use crate::interpreter::engine::VariantKind;
use crate::parser::Type;
use serde::de::{self, DeserializeSeed, Deserializer, EnumAccess, IgnoredAny, MapAccess, SeqAccess, VariantAccess, Visitor};
use serde::ser::{self, Serialize, SerializeMap, SerializeSeq, Serializer};
use std::collections::BTreeMap;
use std::fmt;

// ===================================================================== encode

/// A runtime value seen through serde's data model (serde derive's default externally tagged encoding).
pub(super) struct Enc<'a> {
    pub v: &'a Value,
    /// Host objects (`json::Value`) know their own encoding.
    pub native: fn(&Value) -> Option<serde_json::Value>,
    /// A struct's wire field names (`@json(rename = ..)`), by struct name.
    pub wire: &'a dyn Fn(&str) -> Option<Rc<[&'static str]>>,
}

impl<'a> Enc<'a> {
    fn at<'b>(&self, v: &'b Value) -> Enc<'b>
    where
        'a: 'b,
    {
        Enc { v, native: self.native, wire: self.wire }
    }
}

impl Serialize for Enc<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.v {
            Value::Unit => s.serialize_unit(),
            Value::Bool(b) => s.serialize_bool(*b),
            Value::Int(n, t) => int(s, *n, *t),
            Value::Float(f, FloatTy::F32) => s.serialize_f32(*f as f32),
            Value::Float(f, _) => s.serialize_f64(*f),
            Value::Char(c) => s.serialize_char(*c),
            Value::Str(t) => s.serialize_str(t),
            Value::Vec(_) | Value::Tuple(_) => self.seq(s, self.v.items().iter()),
            Value::Set(m) => self.seq(s, m.keys().map(|k| &k.0)),
            Value::Map(m) => {
                let mut o = s.serialize_map(None)?;
                for (k, x) in m.iter() {
                    o.serialize_entry(&self.at(&k.0), &self.at(x))?;
                }
                o.end()
            }
            Value::Struct(sv) => {
                let positional = !sv.fields.is_empty() && sv.fields.iter().enumerate().all(|(i, (n, _))| **n == i.to_string());
                if positional && sv.fields.len() == 1 {
                    return self.at(&sv.fields[0].1).serialize(s);
                }
                if positional {
                    return self.seq(s, sv.fields.iter().map(|(_, x)| x));
                }
                let wire = (self.wire)(&sv.name).filter(|w| w.len() == sv.fields.len());
                let mut o = s.serialize_map(Some(sv.fields.len()))?;
                for (i, (n, x)) in sv.fields.iter().enumerate() {
                    let key: &str = wire.as_ref().map_or(&**n, |w| w[i]);
                    o.serialize_entry(key, &self.at(x))?;
                }
                o.end()
            }
            Value::Enum(e) => match (&*e.ty, &e.payload) {
                ("Option", Payload::Tuple(p)) if e.idx == 1 => s.serialize_some(&self.at(&p[0])),
                ("Option", _) => s.serialize_none(),
                (_, Payload::Unit) => s.serialize_str(&e.variant),
                (_, Payload::Tuple(p)) => {
                    let mut o = s.serialize_map(Some(1))?;
                    if p.len() == 1 {
                        o.serialize_entry(&*e.variant, &self.at(&p[0]))?;
                    } else {
                        o.serialize_entry(&*e.variant, &Items(p.iter().map(|x| self.at(x)).collect()))?;
                    }
                    o.end()
                }
                (_, Payload::Struct(fs)) => {
                    let inner: Vec<(&str, Enc)> = fs.iter().map(|(n, x)| (&**n, self.at(x))).collect();
                    let mut o = s.serialize_map(Some(1))?;
                    o.serialize_entry(&*e.variant, &Fields(inner))?;
                    o.end()
                }
            },
            Value::Ref(p) => self.at(&p.read().map_err(|e| ser::Error::custom(format!("{e:?}")))?).serialize(s),
            Value::Rc(slot) => {
                let inner = slot.borrow().clone();
                self.at(&inner).serialize(s)
            }
            other => match (self.native)(other) {
                Some(j) => j.serialize(s),
                None => Err(ser::Error::custom(format!("cannot encode a {} as JSON", other.type_name()))),
            },
        }
    }
}

impl Enc<'_> {
    fn seq<'b, S: Serializer>(&self, s: S, xs: impl Iterator<Item = &'b Value>) -> Result<S::Ok, S::Error> {
        let xs: Vec<&Value> = xs.collect();
        let mut q = s.serialize_seq(Some(xs.len()))?;
        for x in xs {
            q.serialize_element(&self.at(x))?;
        }
        q.end()
    }
}

struct Items<'a>(Vec<Enc<'a>>);
impl Serialize for Items<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut q = s.serialize_seq(Some(self.0.len()))?;
        for x in &self.0 {
            q.serialize_element(x)?;
        }
        q.end()
    }
}

struct Fields<'a>(Vec<(&'a str, Enc<'a>)>);
impl Serialize for Fields<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut o = s.serialize_map(Some(self.0.len()))?;
        for (n, x) in &self.0 {
            o.serialize_entry(n, x)?;
        }
        o.end()
    }
}

fn int<S: Serializer>(s: S, n: i128, t: IntTy) -> Result<S::Ok, S::Error> {
    match t {
        IntTy::I8 => s.serialize_i8(n as i8),
        IntTy::I16 => s.serialize_i16(n as i16),
        IntTy::I32 => s.serialize_i32(n as i32),
        IntTy::I128 => s.serialize_i128(n),
        IntTy::U8 => s.serialize_u8(n as u8),
        IntTy::U16 => s.serialize_u16(n as u16),
        IntTy::U32 => s.serialize_u32(n as u32),
        IntTy::U64 | IntTy::Usize => s.serialize_u64(n as u64),
        IntTy::U128 => s.serialize_u128(n as u128),
        IntTy::I64 | IntTy::Isize | IntTy::Lit => s.serialize_i64(n as i64),
    }
}

/// `json.to_string(v)` / `json.to_string_pretty(v)`.
pub(super) fn encode(
    v: &Value,
    pretty: bool,
    native: fn(&Value) -> Option<serde_json::Value>,
    wire: &dyn Fn(&str) -> Option<Rc<[&'static str]>>,
) -> Result<String, String> {
    let e = Enc { v, native, wire };
    if pretty { serde_json::to_string_pretty(&e) } else { serde_json::to_string(&e) }.map_err(|e| e.to_string())
}

// ===================================================================== decode

/// `json.parse_string::<T>(text)`: the same pipeline as `windjammer_runtime::json::parse_string`.
pub(super) fn decode(it: &mut Interpreter, text: &str, ty: &Type) -> Result<Value, String> {
    let mut de = serde_json::Deserializer::from_str(text);
    let mut track = serde_path_to_error::Track::new();
    let out = Dec { it, ty }.deserialize(serde_path_to_error::Deserializer::new(&mut de, &mut track));
    match out {
        Ok(v) => de.end().map(|_| v).map_err(|e| format!("$: {e}")),
        Err(e) => Err(format!("{}: {e}", json_path(&track.path()))),
    }
}

/// `$`, `$.home.city`, `$.tags[2]`, `$["odd key"]`: the path grammar of ADR-0027 (mirrored in the runtime).
pub(crate) fn json_path(p: &serde_path_to_error::Path) -> String {
    use serde_path_to_error::Segment;
    let mut out = String::from("$");
    for seg in p.iter() {
        match seg {
            Segment::Seq { index } => out.push_str(&format!("[{index}]")),
            Segment::Map { key } | Segment::Enum { variant: key } => {
                if !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    out.push('.');
                    out.push_str(key);
                } else {
                    out.push_str(&format!("[{}]", serde_json::to_string(key).unwrap_or_default()));
                }
            }
            Segment::Unknown => out.push_str(".?"),
        }
    }
    out
}

/// Decode a value of declared type `ty`.
struct Dec<'a> {
    it: &'a mut Interpreter,
    ty: &'a Type,
}

type DeResult<E> = Result<Value, E>;

fn leaf(n: &str) -> &str {
    n.rsplit("::").next().unwrap_or(n)
}

fn args_of(t: &Type) -> Vec<Type> {
    match t {
        Type::Parameterized(_, a) => a.clone(),
        Type::Vec(e) => vec![(**e).clone()],
        _ => vec![],
    }
}

/// serde's `OneOf` rendering of the expected names (`unknown field` / `unknown variant` messages).
/// `de::Error::unknown_field` wants `&'static` lists; serde_json's default implementation is exactly
/// `custom("unknown field `x`, expected ...")`, which this reproduces from the program's own names.
fn one_of(names: &[&str]) -> String {
    match names {
        [] => "there are no fields".to_string(),
        [a] => format!("expected `{a}`"),
        [a, b] => format!("expected `{a}` or `{b}`"),
        _ => format!("expected one of {}", names.iter().map(|n| format!("`{n}`")).collect::<Vec<_>>().join(", ")),
    }
}

fn unknown_field<E: de::Error>(v: &str, names: &[&str]) -> E {
    E::custom(format_args!("unknown field `{v}`, {}", one_of(names)))
}

fn unknown_variant<E: de::Error>(v: &str, names: &[&str]) -> E {
    if names.is_empty() {
        E::custom(format_args!("unknown variant `{v}`, there are no variants"))
    } else {
        E::custom(format_args!("unknown variant `{v}`, {}", one_of(names)))
    }
}

/// Type and field-name hints passed to `deserialize_struct`/`deserialize_enum`: serde_json ignores them.
const NO_NAMES: &[&str] = &[];

impl<'de> DeserializeSeed<'de> for Dec<'_> {
    type Value = Value;

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> DeResult<D::Error> {
        use serde::Deserialize;
        if let Some(t) = IntTy::from_type(self.ty) {
            return Ok(match t {
                IntTy::I8 => Value::Int(i8::deserialize(d)? as i128, t),
                IntTy::I16 => Value::Int(i16::deserialize(d)? as i128, t),
                IntTy::I32 => Value::Int(i32::deserialize(d)? as i128, t),
                IntTy::I128 => Value::Int(i128::deserialize(d)?, t),
                IntTy::Isize => Value::Int(isize::deserialize(d)? as i128, t),
                IntTy::U8 => Value::Int(u8::deserialize(d)? as i128, t),
                IntTy::U16 => Value::Int(u16::deserialize(d)? as i128, t),
                IntTy::U32 => Value::Int(u32::deserialize(d)? as i128, t),
                IntTy::U64 => Value::Int(u64::deserialize(d)? as i128, t),
                IntTy::Usize => Value::Int(usize::deserialize(d)? as i128, t),
                IntTy::U128 => Value::Int(u128::deserialize(d)? as i128, t),
                IntTy::I64 | IntTy::Lit => Value::Int(i64::deserialize(d)? as i128, IntTy::I64),
            });
        }
        if let Some(ft) = type_is_float(self.ty) {
            return Ok(match ft {
                FloatTy::F32 => Value::Float(f32::deserialize(d)? as f64, ft),
                _ => Value::Float(f64::deserialize(d)?, FloatTy::F64),
            });
        }
        match self.ty {
            Type::Bool => Ok(Value::Bool(bool::deserialize(d)?)),
            Type::String => Ok(Value::str(String::deserialize(d)?)),
            Type::Custom(n) if matches!(leaf(n), "char") => Ok(Value::Char(char::deserialize(d)?)),
            Type::Reference(i) | Type::MutableReference(i) => Dec { it: self.it, ty: i }.deserialize(d),
            Type::Option(e) => d.deserialize_option(OptionV { it: self.it, ty: e }),
            Type::Vec(e) => d.deserialize_seq(SeqV { it: self.it, ty: e }),
            Type::Tuple(ts) if ts.is_empty() => {
                <()>::deserialize(d)?;
                Ok(Value::Unit)
            }
            Type::Tuple(ts) => d.deserialize_tuple(ts.len(), TupleV { it: self.it, tys: ts, what: format!("a tuple of size {}", ts.len()) }),
            Type::Parameterized(n, _) | Type::Custom(n) if matches!(leaf(n), "Vec" | "VecDeque") => {
                let e = args_of(self.ty).first().cloned().unwrap_or(Type::Infer);
                d.deserialize_seq(SeqV { it: self.it, ty: &e })
            }
            Type::Parameterized(n, args) if leaf(n).ends_with("Map") => {
                let (k, v) = (args.first().cloned().unwrap_or(Type::String), args.get(1).cloned().unwrap_or(Type::Infer));
                d.deserialize_map(MapV { it: self.it, k: &k, v: &v, hash: leaf(n) == "HashMap" })
            }
            Type::Custom(n) | Type::Parameterized(n, _) => {
                let base = leaf(n).to_string();
                if let Some(def) = self.it.structs.get(&base) {
                    let fields: Vec<(&'static str, Type)> = def.fields.iter().map(|(n, t)| (*n, t.clone())).collect();
                    let (tuple, copy, allow_unknown) = (def.tuple, def.copy, def.allow_unknown);
                    let names = def.wire.clone();
                    let v = StructV { it: self.it, name: &base, fields: &fields, names: &names, copy, allow_unknown };
                    return match (tuple, fields.len()) {
                        (true, 1) => d.deserialize_newtype_struct("", v),
                        (true, n) => d.deserialize_tuple_struct("", n, v),
                        _ => d.deserialize_struct("", NO_NAMES, v),
                    };
                }
                if self.it.enums.contains_key(&base) {
                    let names: Vec<&'static str> = self.it.enums[&base].variants.iter().map(|v| v.name).collect();
                    return d.deserialize_enum("", NO_NAMES, EnumV { it: self.it, name: &base, names: &names });
                }
                Err(de::Error::custom(format!("cannot decode JSON into `{base}`")))
            }
            other => Err(de::Error::custom(format!("cannot decode JSON into {other:?}"))),
        }
    }
}

struct OptionV<'a> {
    it: &'a mut Interpreter,
    ty: &'a Type,
}

impl<'de> Visitor<'de> for OptionV<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("option")
    }
    fn visit_unit<E: de::Error>(self) -> DeResult<E> {
        Ok(Value::none())
    }
    fn visit_none<E: de::Error>(self) -> DeResult<E> {
        Ok(Value::none())
    }
    fn visit_some<D: Deserializer<'de>>(self, d: D) -> DeResult<D::Error> {
        Ok(Value::some(Dec { it: self.it, ty: self.ty }.deserialize(d)?))
    }
}

struct SeqV<'a> {
    it: &'a mut Interpreter,
    ty: &'a Type,
}

impl<'de> Visitor<'de> for SeqV<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a sequence")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> DeResult<A::Error> {
        let mut out = Vec::new();
        while let Some(x) = a.next_element_seed(Dec { it: &mut *self.it, ty: self.ty })? {
            out.push(x);
        }
        Ok(Value::vec(out))
    }
}

struct TupleV<'a> {
    it: &'a mut Interpreter,
    tys: &'a [Type],
    what: String,
}

impl<'de> Visitor<'de> for TupleV<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.what)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> DeResult<A::Error> {
        let mut out = Vec::new();
        for (i, t) in self.tys.iter().enumerate() {
            match a.next_element_seed(Dec { it: &mut *self.it, ty: t })? {
                Some(x) => out.push(x),
                None => return Err(de::Error::invalid_length(i, &self)),
            }
        }
        Ok(Value::tuple(out))
    }
}

struct MapV<'a> {
    it: &'a mut Interpreter,
    k: &'a Type,
    v: &'a Type,
    hash: bool,
}

impl<'de> Visitor<'de> for MapV<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a map")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> DeResult<A::Error> {
        let mut m = BTreeMap::new();
        while let Some(k) = a.next_key_seed(Dec { it: &mut *self.it, ty: self.k })? {
            let v = a.next_value_seed(Dec { it: &mut *self.it, ty: self.v })?;
            m.insert(Key(k), v);
        }
        Ok(if self.hash { Value::hash_map(m) } else { Value::btree_map(m) })
    }
}

/// A struct, as `#[derive(Deserialize)]` + `#[serde(deny_unknown_fields)]` decodes it.
struct StructV<'a> {
    it: &'a mut Interpreter,
    name: &'a str,
    fields: &'a [(&'static str, Type)],
    names: &'a [&'static str],
    copy: bool,
    allow_unknown: bool,
}

impl StructV<'_> {
    fn build(self, vals: Vec<Value>) -> Value {
        let named = self.fields.iter().map(|(n, _)| Rc::from(*n)).zip(vals).collect();
        self.it.make_struct(self.name, named, self.copy)
    }
}

impl<'de> Visitor<'de> for StructV<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let positional = self.fields.first().is_some_and(|(n, _)| *n == "0");
        match (positional, self.fields.len()) {
            (true, 1) => write!(f, "tuple struct {}", self.name),
            (true, _) => write!(f, "tuple struct {}", self.name),
            _ => write!(f, "struct {}", self.name),
        }
    }
    fn visit_newtype_struct<D: Deserializer<'de>>(self, d: D) -> DeResult<D::Error> {
        let ty = self.fields[0].1.clone();
        let v = Dec { it: &mut *self.it, ty: &ty }.deserialize(d)?;
        Ok(self.build(vec![v]))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> DeResult<A::Error> {
        let mut vals = Vec::new();
        for (i, (_, t)) in self.fields.iter().enumerate() {
            match a.next_element_seed(Dec { it: &mut *self.it, ty: t })? {
                Some(x) => vals.push(x),
                None => {
                    let what = format!("{} with {} elements", self.expecting_text(), self.fields.len());
                    return Err(de::Error::invalid_length(i, &what.as_str()));
                }
            }
        }
        Ok(self.build(vals))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> DeResult<A::Error> {
        let mut slots: Vec<Option<Value>> = vec![None; self.fields.len()];
        while let Some(key) = a.next_key_seed(FieldSeed { names: self.names, allow_unknown: self.allow_unknown })? {
            let Some(i) = key else {
                a.next_value::<IgnoredAny>()?;
                continue;
            };
            if slots[i].is_some() {
                return Err(de::Error::duplicate_field(self.names[i]));
            }
            slots[i] = Some(a.next_value_seed(Dec { it: &mut *self.it, ty: &self.fields[i].1 })?);
        }
        let mut vals = Vec::new();
        for ((slot, (_, t)), wire) in slots.into_iter().zip(self.fields).zip(self.names) {
            vals.push(match slot {
                Some(v) => v,
                None if matches!(t, Type::Option(_)) => Value::none(),
                None => return Err(de::Error::missing_field(wire)),
            });
        }
        Ok(self.build(vals))
    }
}

impl StructV<'_> {
    fn expecting_text(&self) -> String {
        if self.fields.first().is_some_and(|(n, _)| *n == "0") { format!("tuple struct {}", self.name) } else { format!("struct {}", self.name) }
    }
}

/// A struct's field name (its index); an unknown name is an error unless `@json(allow_unknown)`
/// (ADR-0027: deny by default), in which case it is `None` and its value is skipped.
struct FieldSeed<'a> {
    names: &'a [&'static str],
    allow_unknown: bool,
}

impl<'de> DeserializeSeed<'de> for FieldSeed<'_> {
    type Value = Option<usize>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Option<usize>, D::Error> {
        d.deserialize_identifier(self)
    }
}

impl<'de> Visitor<'de> for FieldSeed<'_> {
    type Value = Option<usize>;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("field identifier")
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Option<usize>, E> {
        match self.names.iter().position(|n| *n == v) {
            Some(i) => Ok(Some(i)),
            None if self.allow_unknown => Ok(None),
            None => Err(unknown_field(v, self.names)),
        }
    }
}

/// An enum in serde's externally tagged form.
struct EnumV<'a> {
    it: &'a mut Interpreter,
    name: &'a str,
    names: &'a [&'static str],
}

struct VariantSeed<'a> {
    names: &'a [&'static str],
}

impl<'de> DeserializeSeed<'de> for VariantSeed<'_> {
    type Value = usize;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<usize, D::Error> {
        d.deserialize_identifier(self)
    }
}

impl<'de> Visitor<'de> for VariantSeed<'_> {
    type Value = usize;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("variant identifier")
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<usize, E> {
        self.names.iter().position(|n| *n == v).ok_or_else(|| unknown_variant(v, self.names))
    }
}

impl<'de> Visitor<'de> for EnumV<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "enum {}", self.name)
    }
    fn visit_enum<A: EnumAccess<'de>>(self, a: A) -> DeResult<A::Error> {
        let (idx, va) = a.variant_seed(VariantSeed { names: self.names })?;
        let kind = match &self.it.enums[self.name].variants[idx].kind {
            VariantKind::Unit => None,
            VariantKind::Tuple(ts) => Some((ts.clone(), Vec::new())),
            VariantKind::Struct(fs) => Some((fs.iter().map(|(_, t)| t.clone()).collect(), fs.iter().map(|(n, _)| *n).collect::<Vec<_>>())),
        };
        let mk = |it: &mut Interpreter, vals| it.make_variant(self.name, idx, vals).map_err(|e| de::Error::custom(format!("{e:?}")));
        match kind {
            None => {
                va.unit_variant()?;
                mk(self.it, vec![])
            }
            Some((ts, names)) if names.is_empty() && ts.len() == 1 => {
                let v = va.newtype_variant_seed(Dec { it: &mut *self.it, ty: &ts[0] })?;
                mk(self.it, vec![v])
            }
            Some((ts, names)) if names.is_empty() => {
                let what = format!("tuple variant {}::{}", self.name, self.names[idx]);
                let v = va.tuple_variant(ts.len(), TupleV { it: &mut *self.it, tys: &ts, what })?;
                mk(self.it, v.items().to_vec())
            }
            Some((ts, names)) => {
                let fields: Vec<(&'static str, Type)> = names.into_iter().zip(ts).collect();
                let fnames: Vec<&'static str> = fields.iter().map(|(n, _)| *n).collect();
                let label = format!("struct variant {}::{}", self.name, self.names[idx]);
                let v = va.struct_variant(NO_NAMES, VariantFieldsV { it: &mut *self.it, label, fields: &fields, names: &fnames })?;
                let Value::Tuple(vals) = v else { return mk(self.it, vec![v]) };
                let payload = fields.iter().map(|(n, _)| Rc::from(*n)).zip(vals.iter().cloned()).collect();
                let vname = self.it.enums[self.name].variants[idx].name;
                Ok(Value::enum_val(self.name, vname, idx as u32, Payload::Struct(payload)))
            }
        }
    }
    fn visit_str<E: de::Error>(self, v: &str) -> DeResult<E> {
        let idx = self.names.iter().position(|n| *n == v).ok_or_else(|| unknown_variant(v, self.names))?;
        self.it.make_variant(self.name, idx, vec![]).map_err(|e| E::custom(format!("{e:?}")))
    }
}

/// The fields of a struct variant, decoded into a tuple (the caller names them).
struct VariantFieldsV<'a> {
    it: &'a mut Interpreter,
    label: String,
    fields: &'a [(&'static str, Type)],
    names: &'a [&'static str],
}

impl<'de> Visitor<'de> for VariantFieldsV<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.label)
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> DeResult<A::Error> {
        let mut slots: Vec<Option<Value>> = vec![None; self.fields.len()];
        while let Some(key) = a.next_key_seed(FieldSeed { names: self.names, allow_unknown: false })? {
            let Some(i) = key else { continue };
            if slots[i].is_some() {
                return Err(de::Error::duplicate_field(self.names[i]));
            }
            slots[i] = Some(a.next_value_seed(Dec { it: &mut *self.it, ty: &self.fields[i].1 })?);
        }
        let mut vals = Vec::new();
        for (slot, (n, t)) in slots.into_iter().zip(self.fields) {
            vals.push(match slot {
                Some(v) => v,
                None if matches!(t, Type::Option(_)) => Value::none(),
                None => return Err(de::Error::missing_field(n)),
            });
        }
        Ok(Value::Tuple(Rc::new(vals)))
    }
}
