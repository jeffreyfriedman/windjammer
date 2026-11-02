//! The `durable fn` journal (ADR-0021 sections 9 and E4b): file layout, run identity, step numbering, replay, the `done`
//! record and the divergence message, in one place.
//!
//! This file is compiled twice, from this one source: as `windjammer_runtime::durable_core` (what a compiled program uses)
//! and as `windjammer::interpreter::durable_core` (what the interpreter uses), so the two cannot disagree about the format.
//! Values are `serde_json::Value`; each side brings its own adapter between its values and JSON.
//!
//! A run is one JSON Lines file `<fn>-<digest>.wjlog` in the journal directory:
//! `{"args":[..],"fn":"name","wj_durable":1}`, then `{"callee":..,"result":..,"step":N,"vt_ns":..}` per journaled step, then
//! `{"done":..}` once the function returned. Keys are sorted, so two writers produce identical files.

use serde_json::{json, Value as Json};
use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Canonical text of a JSON value: object keys sorted at every level, no whitespace.
pub fn canon(v: &Json) -> String {
    match v {
        Json::Object(m) => {
            let sorted: BTreeMap<&String, &Json> = m.iter().collect();
            let parts: Vec<String> = sorted.into_iter().map(|(k, v)| format!("{}:{}", Json::String(k.clone()), canon(v))).collect();
            format!("{{{}}}", parts.join(","))
        }
        Json::Array(a) => format!("[{}]", a.iter().map(canon).collect::<Vec<_>>().join(",")),
        other => other.to_string(),
    }
}

/// 64-bit FNV-1a: a stable, dependency-free digest for run identity and seeds.
pub fn hash64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// The value of an argument without the type tags the implementations put on numbers (`{"i":"lit","v":"3"}` and
/// `{"i":"i64","v":"3"}` are the same argument): what identifies a call.
fn untagged(v: &Json) -> Json {
    match v {
        Json::Object(m) if m.len() == 2 && m.contains_key("i") && m.contains_key("v") => m["v"].clone(),
        Json::Object(m) if m.len() == 2 && m.contains_key("f") && m.contains_key("bits") => m["bits"].clone(),
        Json::Object(m) => Json::Object(m.iter().map(|(k, x)| (k.clone(), untagged(x))).collect()),
        Json::Array(a) => Json::Array(a.iter().map(untagged).collect()),
        other => other.clone(),
    }
}

/// The identity of a call: the function, its arguments and how many earlier calls in this process had the same.
pub fn run_id(fn_qual: &str, args: &[Json], occurrence: usize) -> String {
    let key = format!("{fn_qual}|{}#{occurrence}", canon(&untagged(&Json::Array(args.to_vec()))));
    format!("{}-{:016x}", fn_qual.replace("::", "."), hash64(key.as_bytes()))
}

/// `--crash-after N` / `WJ_CRASH_AFTER`: `N` or `N:mid`.
pub fn parse_crash(text: &str) -> Result<(usize, bool), String> {
    let (n, mid) = match text.split_once(':') {
        Some((n, "mid")) => (n, true),
        Some(_) => return Err(format!("the crash point is N or N:mid, found {text:?}")),
        None => (text, false),
    };
    n.trim().parse::<usize>().map(|n| (n, mid)).map_err(|_| format!("the crash point is N or N:mid, found {text:?}"))
}

pub fn divergence(idx: usize, journaled: &str, now: &str) -> String {
    format!(
        "WJ0049: durable replay diverged at step {idx}: the journal has `{journaled}`, the program now calls `{now}`\n  help: delete the journal to start over, or restore the program that wrote it"
    )
}

/// What to do with the next step.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// The journal has it: return `result` without running anything; `vt_ns` is the virtual clock after the step.
    Replay { idx: usize, result: Json, vt_ns: u64 },
    /// Run it, then [`RunJournal::record`] the result under `idx`.
    Live { idx: usize },
}

pub enum Opened {
    /// The function finished in an earlier process: this is its result.
    Done(Json),
    Run(Box<RunJournal>),
}

struct Entry {
    callee: String,
    result: Json,
    vt_ns: u64,
}

pub struct RunJournal {
    file: Option<std::fs::File>,
    steps: HashMap<usize, Entry>,
    next: usize,
}

impl RunJournal {
    /// Open (and, in `dir`, create) the journal of the run `id`. Without a directory the journal lives in memory.
    pub fn open(dir: Option<&Path>, id: &str, fn_qual: &str, args: &[Json]) -> Result<Opened, String> {
        let mut run = RunJournal { file: None, steps: HashMap::new(), next: 0 };
        let Some(dir) = dir else { return Ok(Opened::Run(Box::new(run))) };
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create the journal directory {}: {e}", dir.display()))?;
        let path: PathBuf = dir.join(format!("{id}.wjlog"));
        if let Ok(text) = std::fs::read_to_string(&path) {
            for line in text.lines().skip(1) {
                // a line cut short by a crash is simply not there
                let Ok(j) = serde_json::from_str::<Json>(line) else { continue };
                if let Some(done) = j.get("done") {
                    return Ok(Opened::Done(done.clone()));
                }
                if let Some(n) = j.get("step").and_then(|n| n.as_u64()) {
                    run.steps.insert(
                        n as usize,
                        Entry { callee: j["callee"].as_str().unwrap_or("").to_string(), result: j["result"].clone(), vt_ns: j["vt_ns"].as_u64().unwrap_or(0) },
                    );
                }
            }
        }
        let fresh = !path.exists();
        let torn = std::fs::read(&path).map(|b| !b.is_empty() && b.last() != Some(&b'\n')).unwrap_or(false);
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path).map_err(|e| format!("cannot open the journal {}: {e}", path.display()))?;
        if torn {
            // the last line was cut short by a crash: end it, so the next record starts on a line of its own
            writeln!(file).map_err(|e| e.to_string())?;
        }
        if fresh {
            writeln!(file, "{}", canon(&json!({ "wj_durable": 1, "fn": fn_qual, "args": args }))).map_err(|e| e.to_string())?;
        }
        run.file = Some(file);
        Ok(Opened::Run(Box::new(run)))
    }

    /// Number the next step (steps are numbered when they start, so a step containing steps replays correctly).
    pub fn next_step(&mut self, callee: &str) -> Result<Step, String> {
        let idx = self.next;
        self.next += 1;
        match self.steps.get(&idx) {
            Some(e) if e.callee != callee => Err(divergence(idx, &e.callee, callee)),
            Some(e) => Ok(Step::Replay { idx, result: e.result.clone(), vt_ns: e.vt_ns }),
            None => Ok(Step::Live { idx }),
        }
    }

    pub fn record(&mut self, idx: usize, callee: &str, result: Json, vt_ns: u64) -> Result<(), String> {
        if let Some(f) = self.file.as_mut() {
            let line = canon(&json!({ "step": idx, "callee": callee, "result": result, "vt_ns": vt_ns }));
            writeln!(f, "{line}").and_then(|_| f.flush()).map_err(|e| format!("cannot write the journal: {e}"))?;
        }
        self.steps.insert(idx, Entry { callee: callee.to_string(), result, vt_ns });
        Ok(())
    }

    /// The function returned `result`: mark the run finished.
    pub fn finish(mut self, result: &Json) -> Result<(), String> {
        if let Some(f) = self.file.as_mut() {
            writeln!(f, "{}", canon(&json!({ "done": result }))).and_then(|_| f.flush()).map_err(|e| format!("cannot write the journal: {e}"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_text_sorts_keys() {
        assert_eq!(canon(&json!({"b": [1, {"y": 1, "x": 2}], "a": null})), r#"{"a":null,"b":[1,{"x":2,"y":1}]}"#);
    }

    #[test]
    fn identity_is_stable_and_separates_calls() {
        let a = run_id("m::f", &[json!(1)], 0);
        assert_eq!(a, run_id("m::f", &[json!(1)], 0));
        assert_ne!(a, run_id("m::f", &[json!(2)], 0));
        assert_ne!(a, run_id("m::f", &[json!(1)], 1));
        // the type tag a number carries does not change which call it is
        let lit = run_id("m::f", &[json!({"i": "lit", "v": "3"})], 0);
        assert_eq!(lit, run_id("m::f", &[json!({"i": "i64", "v": "3"})], 0));
        assert_ne!(lit, run_id("m::f", &[json!({"i": "i64", "v": "4"})], 0));
        assert!(a.starts_with("m.f-") && a.len() == "m.f-".len() + 16, "{a}");
    }

    #[test]
    fn a_run_replays_what_it_recorded_and_resumes_after_it() {
        let d = std::env::temp_dir().join(format!("wj_durable_core_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let Opened::Run(mut r) = RunJournal::open(Some(&d), "f-1", "f", &[json!(7)]).unwrap() else { panic!("fresh") };
        assert_eq!(r.next_step("a").unwrap(), Step::Live { idx: 0 });
        r.record(0, "a", json!("x"), 5).unwrap();
        assert_eq!(r.next_step("b").unwrap(), Step::Live { idx: 1 });
        drop(r); // a crash before step 1 was recorded
        let Opened::Run(mut r) = RunJournal::open(Some(&d), "f-1", "f", &[json!(7)]).unwrap() else { panic!("resume") };
        assert_eq!(r.next_step("a").unwrap(), Step::Replay { idx: 0, result: json!("x"), vt_ns: 5 });
        assert_eq!(r.next_step("b").unwrap(), Step::Live { idx: 1 });
        assert!(RunJournal::open(Some(&d), "f-1", "f", &[json!(7)]).is_ok());
        r.record(1, "b", json!(2), 9).unwrap();
        r.finish(&json!(3)).unwrap();
        assert!(matches!(RunJournal::open(Some(&d), "f-1", "f", &[json!(7)]).unwrap(), Opened::Done(v) if v == json!(3)));
        let text = std::fs::read_to_string(d.join("f-1.wjlog")).unwrap();
        assert_eq!(
            text,
            "{\"args\":[7],\"fn\":\"f\",\"wj_durable\":1}\n{\"callee\":\"a\",\"result\":\"x\",\"step\":0,\"vt_ns\":5}\n{\"callee\":\"b\",\"result\":2,\"step\":1,\"vt_ns\":9}\n{\"done\":3}\n"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_different_callee_is_a_divergence_and_a_cut_line_is_ignored() {
        let d = std::env::temp_dir().join(format!("wj_durable_core_div_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("g-1.wjlog"), "{\"args\":[],\"fn\":\"g\",\"wj_durable\":1}\n{\"callee\":\"a\",\"result\":1,\"step\":0,\"vt_ns\":0}\n{\"callee\":\"b\",\"res").unwrap();
        let Opened::Run(mut r) = RunJournal::open(Some(&d), "g-1", "g", &[]).unwrap() else { panic!() };
        let e = r.next_step("other").unwrap_err();
        assert!(e.starts_with("WJ0049: durable replay diverged at step 0") && e.contains("`a`") && e.contains("`other`"), "{e}");
        assert!(matches!(r.next_step("b").unwrap(), Step::Live { idx: 1 }), "the cut line is not a step");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn crash_points_parse() {
        assert_eq!(parse_crash("3").unwrap(), (3, false));
        assert_eq!(parse_crash("2:mid").unwrap(), (2, true));
        assert!(parse_crash("x").is_err() && parse_crash("1:late").is_err());
    }

    /// `tests/effects/durable_vectors.json`: the same scenarios run against the core as compiled into the runtime and into the
    /// interpreter (this test exists in both builds), so the two cannot differ on the file format or on replay.
    #[test]
    fn the_vectors_hold() {
        let all: Json = serde_json::from_str(include_str!("../../../../tests/effects/durable_vectors.json")).unwrap();
        for (n, case) in all.as_array().unwrap().iter().enumerate() {
            let name = case["name"].as_str().unwrap();
            let d = std::env::temp_dir().join(format!("wj_durable_vec_{}_{n}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            let file = d.join("v-1.wjlog");
            if let Some(lines) = case["journal"].as_array() {
                let text: Vec<&str> = lines.iter().map(|l| l.as_str().unwrap()).collect();
                std::fs::write(&file, text.join("\n")).unwrap();
            }
            let args: Vec<Json> = case["args"].as_array().unwrap().clone();
            let mut got: Vec<String> = Vec::new();
            match RunJournal::open(Some(&d), "v-1", "f", &args).unwrap() {
                Opened::Done(r) => got.push(format!("done:{}", canon(&r))),
                Opened::Run(mut run) => {
                    for call in case["calls"].as_array().unwrap() {
                        let callee = call["callee"].as_str().unwrap();
                        match run.next_step(callee) {
                            Ok(Step::Replay { result, .. }) => got.push(format!("replay:{}", canon(&result))),
                            Ok(Step::Live { idx }) => {
                                got.push("live".to_string());
                                run.record(idx, callee, call["live"].clone(), 0).unwrap();
                            }
                            Err(m) => {
                                got.push(format!("diverge:{}", m.lines().next().unwrap_or("")));
                                break;
                            }
                        }
                    }
                    if let Some(f) = case.get("finish") {
                        run.finish(f).unwrap();
                    }
                }
            }
            let want: Vec<String> = case["outcome"].as_array().unwrap().iter().map(|o| o.as_str().unwrap().to_string()).collect();
            assert_eq!(got, want, "{name}");
            if let Some(lines) = case.get("file").and_then(|f| f.as_array()) {
                let text = std::fs::read_to_string(&file).unwrap();
                let want: Vec<&str> = lines.iter().map(|l| l.as_str().unwrap()).collect();
                assert_eq!(text.lines().collect::<Vec<_>>(), want, "{name}: the file");
            }
            let _ = std::fs::remove_dir_all(&d);
        }
    }
}
