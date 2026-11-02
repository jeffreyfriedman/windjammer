//! `durable fn` for compiled programs (ADR-0021 stage E4b).
//!
//! The compiler lowers a `durable fn` to `durable::run("name", [argument JSON], move || -> T { body })` and each step call to
//! `durable::step("callee", || call)`. The journal is `durable_core`, the same source the interpreter uses, so a journal
//! written by one is read by the other. Values cross as JSON in the interpreter's encoding ([`Journalable`]).
//!
//! * `WJ_JOURNAL_DIR`  where the journals live (default: in memory)
//! * `WJ_CRASH_AFTER`  `N` or `N:mid`: exit with status 137 after the N-th journaled step (or, with `:mid`, after performing it
//!   and before journaling it): the way tests exercise recovery
//!
//! A resumed compiled program gets the journaled values back; unlike the interpreter in a deterministic world it does not
//! re-derive random numbers or restore a virtual clock, because it has neither.

use crate::durable_core::{self as jc, Opened, RunJournal, Step};
use serde_json::{json, Value as Json};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

pub use serde_json::Value as JsonValue;

/// A value that can be written to and read back from a journal.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be journaled by a `durable fn`",
    label = "a step result, an argument or the result of a durable function must be a number, bool, char, string, list, option, result or tuple of those",
    note = "do the work inside the step and return data, or see ADR-0021 (E4b) for what compiled builds journal"
)]
pub trait Journalable {
    fn to_json(&self) -> Json;
    fn from_json(j: &Json) -> Result<Self, String>
    where
        Self: Sized;
}

pub fn to_json<T: Journalable + ?Sized>(v: &T) -> Json {
    v.to_json()
}

fn bad(what: &str, j: &Json) -> String {
    format!("the journal holds {j} where {what} is expected")
}

impl<T: Journalable + ?Sized> Journalable for &T {
    fn to_json(&self) -> Json {
        (**self).to_json()
    }
    fn from_json(j: &Json) -> Result<Self, String> {
        Err(bad("a reference (references cannot be read back)", j))
    }
}

impl Journalable for () {
    fn to_json(&self) -> Json {
        Json::Null
    }
    fn from_json(j: &Json) -> Result<Self, String> {
        j.is_null().then_some(()).ok_or_else(|| bad("()", j))
    }
}

impl Journalable for bool {
    fn to_json(&self) -> Json {
        Json::Bool(*self)
    }
    fn from_json(j: &Json) -> Result<Self, String> {
        j.as_bool().ok_or_else(|| bad("a bool", j))
    }
}

impl Journalable for String {
    fn to_json(&self) -> Json {
        Json::String(self.clone())
    }
    fn from_json(j: &Json) -> Result<Self, String> {
        j.as_str().map(String::from).ok_or_else(|| bad("a string", j))
    }
}

impl Journalable for str {
    fn to_json(&self) -> Json {
        Json::String(self.to_string())
    }
}

impl Journalable for char {
    fn to_json(&self) -> Json {
        json!({ "c": self.to_string() })
    }
    fn from_json(j: &Json) -> Result<Self, String> {
        j.get("c").and_then(|c| c.as_str()).and_then(|s| s.chars().next()).ok_or_else(|| bad("a char", j))
    }
}

macro_rules! ints {
    ($($t:ty => $n:literal),*) => {$(
        impl Journalable for $t {
            fn to_json(&self) -> Json {
                json!({ "i": $n, "v": self.to_string() })
            }
            fn from_json(j: &Json) -> Result<Self, String> {
                // any integer tag is accepted: the interpreter tags a literal `lit` and an `int` `i64`
                let n: i128 = j.get("v").and_then(|v| v.as_str()).and_then(|v| v.parse().ok()).ok_or_else(|| bad(concat!("an ", $n), j))?;
                <$t>::try_from(n).map_err(|_| format!("the journal holds {n}, which does not fit in {}", $n))
            }
        }
    )*};
}
ints!(i8 => "i8", i16 => "i16", i32 => "i32", i64 => "i64", i128 => "i128", isize => "isize", u8 => "u8", u16 => "u16", u32 => "u32", u64 => "u64", u128 => "u128", usize => "usize");

macro_rules! floats {
    ($($t:ty => $n:literal),*) => {$(
        impl Journalable for $t {
            fn to_json(&self) -> Json {
                // the bit pattern is the value: text would lose -0.0 and NaN payloads
                json!({ "f": $n, "bits": format!("{:016x}", (*self as f64).to_bits()) })
            }
            fn from_json(j: &Json) -> Result<Self, String> {
                let bits = j.get("bits").and_then(|b| b.as_str()).and_then(|b| u64::from_str_radix(b, 16).ok()).ok_or_else(|| bad(concat!("a ", $n), j))?;
                Ok(f64::from_bits(bits) as $t)
            }
        }
    )*};
}
floats!(f32 => "f32", f64 => "f64");

impl<T: Journalable> Journalable for Vec<T> {
    fn to_json(&self) -> Json {
        json!({ "vec": self.iter().map(T::to_json).collect::<Vec<_>>() })
    }
    fn from_json(j: &Json) -> Result<Self, String> {
        j.get("vec").and_then(|v| v.as_array()).ok_or_else(|| bad("a list", j))?.iter().map(T::from_json).collect()
    }
}

fn enum_json(ty: &str, var: &str, idx: u32, payload: Option<Json>) -> Json {
    json!({ "en": ty, "var": var, "idx": idx, "p": payload.map(|p| json!({ "t": [p] })).unwrap_or(Json::Null) })
}

fn payload_of<'a>(j: &'a Json, ty: &str, var: &str) -> Option<&'a Json> {
    (j.get("en")?.as_str()? == ty && j.get("var")?.as_str()? == var).then_some(())?;
    j.get("p")?.get("t")?.as_array()?.first()
}

impl<T: Journalable> Journalable for Option<T> {
    fn to_json(&self) -> Json {
        match self {
            Some(v) => enum_json("Option", "Some", 1, Some(v.to_json())),
            None => enum_json("Option", "None", 0, None),
        }
    }
    fn from_json(j: &Json) -> Result<Self, String> {
        if let Some(p) = payload_of(j, "Option", "Some") {
            return T::from_json(p).map(Some);
        }
        if j.get("en").and_then(|e| e.as_str()) == Some("Option") && j.get("var").and_then(|e| e.as_str()) == Some("None") {
            return Ok(None);
        }
        Err(bad("an Option", j))
    }
}

impl<T: Journalable, E: Journalable> Journalable for Result<T, E> {
    fn to_json(&self) -> Json {
        match self {
            Ok(v) => enum_json("Result", "Ok", 0, Some(v.to_json())),
            Err(e) => enum_json("Result", "Err", 1, Some(e.to_json())),
        }
    }
    fn from_json(j: &Json) -> Result<Self, String> {
        if let Some(p) = payload_of(j, "Result", "Ok") {
            return T::from_json(p).map(Ok);
        }
        if let Some(p) = payload_of(j, "Result", "Err") {
            return E::from_json(p).map(Err);
        }
        Err(bad("a Result", j))
    }
}

macro_rules! tuples {
    ($(($($n:tt $t:ident),+)),*) => {$(
        impl<$($t: Journalable),+> Journalable for ($($t,)+) {
            fn to_json(&self) -> Json {
                json!({ "tup": [$(self.$n.to_json()),+] })
            }
            fn from_json(j: &Json) -> Result<Self, String> {
                let items = j.get("tup").and_then(|t| t.as_array()).ok_or_else(|| bad("a tuple", j))?;
                Ok(($($t::from_json(items.get($n).ok_or_else(|| bad("a longer tuple", j))?)?,)+))
            }
        }
    )*};
}
tuples!((0 A, 1 B), (0 A, 1 B, 2 C), (0 A, 1 B, 2 C, 3 D));

#[derive(Default)]
struct State {
    configured: bool,
    dir: Option<PathBuf>,
    crash_after: Option<usize>,
    crash_mid: bool,
    appended: usize,
    occurrences: HashMap<String, usize>,
    runs: Vec<RunJournal>,
    in_step: u32,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn die(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1)
}

fn crash() -> ! {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    std::process::exit(137)
}

/// Run `body` as the durable function `name` with `args` (the JSON of its arguments): resume its journal when it has one.
pub fn run<T: Journalable>(name: &str, args: Vec<Json>, body: impl FnOnce() -> T) -> T {
    let opened = STATE.with(|s| {
        let mut s = s.borrow_mut();
        if !s.configured {
            s.configured = true;
            s.dir = std::env::var_os("WJ_JOURNAL_DIR").map(PathBuf::from);
            if let Ok(c) = std::env::var("WJ_CRASH_AFTER") {
                match jc::parse_crash(&c) {
                    Ok((n, mid)) => {
                        s.crash_after = Some(n);
                        s.crash_mid = mid;
                    }
                    Err(m) => die(&format!("WJ_CRASH_AFTER: {m}")),
                }
            }
        }
        let key = format!("{name}|{}", jc::canon(&Json::Array(args.clone())));
        let occurrence = {
            let n = s.occurrences.entry(key).or_insert(0);
            *n += 1;
            *n - 1
        };
        let id = jc::run_id(name, &args, occurrence);
        RunJournal::open(s.dir.as_deref(), &id, name, &args).unwrap_or_else(|m| die(&m))
    });
    let journal = match opened {
        Opened::Done(result) => return T::from_json(&result).unwrap_or_else(|m| die(&format!("durable fn `{name}`: {m}"))),
        Opened::Run(j) => *j,
    };
    STATE.with(|s| s.borrow_mut().runs.push(journal));
    let result = body();
    let journal = STATE.with(|s| s.borrow_mut().runs.pop()).expect("durable run");
    journal.finish(&result.to_json()).unwrap_or_else(|m| die(&m));
    result
}

/// One step of the running durable function: replayed from the journal, or run and journaled.
pub fn step<T: Journalable>(callee: &str, call: impl FnOnce() -> T) -> T {
    let planned = STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.in_step > 0 || s.runs.is_empty() {
            return None; // a step inside a step, or a call outside any durable function: just a call
        }
        let step = s.runs.last_mut().expect("durable run").next_step(callee).unwrap_or_else(|m| die(&m));
        Some(step)
    });
    let idx = match planned {
        None => return call(),
        Some(Step::Replay { result, .. }) => return T::from_json(&result).unwrap_or_else(|m| die(&format!("step `{callee}`: {m}"))),
        Some(Step::Live { idx }) => idx,
    };
    STATE.with(|s| s.borrow_mut().in_step += 1);
    let value = call();
    STATE.with(|s| s.borrow_mut().in_step -= 1);
    let (crash_before, crash_after) = STATE.with(|s| {
        let s = s.borrow();
        let hit = s.crash_after == Some(s.appended + 1);
        (hit && s.crash_mid, hit && !s.crash_mid)
    });
    if crash_before {
        crash();
    }
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.runs.last_mut().expect("durable run").record(idx, callee, value.to_json(), 0).unwrap_or_else(|m| die(&m));
        s.appended += 1;
    });
    if crash_after {
        crash();
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The value encodings are the interpreter's (`effect_port::codec`): `tests/effects/durable_value_vectors.json` pins both.
    #[test]
    fn values_encode_as_the_vectors_say() {
        let v: Json = serde_json::from_str(include_str!("../../../tests/effects/durable_value_vectors.json")).unwrap();
        let mut seen = 0;
        for case in v.as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let want = &case["json"];
            let got = match name {
                "unit" => ().to_json(),
                "bool" => true.to_json(),
                "int i64" => 5i64.to_json(),
                "negative int" => (-12i64).to_json(),
                "int u8" => 200u8.to_json(),
                "float" => 0.1f64.to_json(),
                "negative zero" => (-0.0f64).to_json(),
                "char" => 'é'.to_json(),
                "string" => "a \"quoted\"\nline".to_string().to_json(),
                "list" => vec![1i64, 2, 3].to_json(),
                "tuple" => ("x".to_string(), 7i64).to_json(),
                "some" => Some("s".to_string()).to_json(),
                "none" => None::<i64>.to_json(),
                "ok" => Ok::<i64, String>(3).to_json(),
                "err" => Err::<i64, String>("no".to_string()).to_json(),
                "nested" => vec![Some((1i64, true)), None].to_json(),
                other => panic!("no Rust value for the vector `{other}`"),
            };
            assert_eq!(&got, want, "{name}");
            seen += 1;
        }
        assert_eq!(seen, 16);
    }

    #[test]
    fn values_decode_from_the_vectors_and_from_other_integer_tags() {
        assert_eq!(i64::from_json(&json!({"i": "lit", "v": "5"})).unwrap(), 5);
        assert_eq!(u8::from_json(&json!({"i": "i64", "v": "200"})).unwrap(), 200);
        assert!(u8::from_json(&json!({"i": "i64", "v": "300"})).is_err());
        assert_eq!(<(String, i64)>::from_json(&("x".to_string(), 7i64).to_json()).unwrap(), ("x".to_string(), 7));
        assert_eq!(Vec::<Option<i64>>::from_json(&vec![Some(1i64), None].to_json()).unwrap(), vec![Some(1), None]);
        assert!(Result::<i64, String>::from_json(&Err::<i64, String>("e".into()).to_json()).unwrap().is_err());
        assert!(f64::from_json(&(-0.0f64).to_json()).unwrap().is_sign_negative());
    }

    #[test]
    fn a_durable_run_journals_steps_and_a_second_call_replays_them() {
        let d = std::env::temp_dir().join(format!("wj_durable_rt_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        STATE.with(|s| *s.borrow_mut() = State { configured: true, dir: Some(d.clone()), ..State::default() });
        let mut live = 0;
        let r = run("work", vec![1i64.to_json()], || {
            let a = step("one", || {
                live += 1;
                10i64
            });
            let b = step("two", || {
                live += 1;
                a + 1
            });
            a + b
        });
        assert_eq!((r, live), (21, 2));
        // the same call again: the run finished, so nothing runs
        STATE.with(|s| s.borrow_mut().occurrences.clear());
        let mut again = 0;
        let r2 = run("work", vec![1i64.to_json()], || {
            again += 1;
            0i64
        });
        assert_eq!((r2, again), (21, 0));
        let text = std::fs::read_to_string(std::fs::read_dir(&d).unwrap().next().unwrap().unwrap().path()).unwrap();
        assert!(text.contains(r#""callee":"one""#) && text.contains(r#""callee":"two""#) && text.trim_end().ends_with(r#"{"done":{"i":"i64","v":"21"}}"#), "{text}");
        let _ = std::fs::remove_dir_all(&d);
    }
}
