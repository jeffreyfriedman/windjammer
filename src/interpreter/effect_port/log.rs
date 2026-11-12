//! Effect log v2 (ADR-0021 section 10; v2 added the capability grants to the header): a JSON Lines file, one header then one event per line, canonical JSON
//! (sorted keys) so two logs of the same run diff cleanly. Payloads over `BLOB_LIMIT` bytes live in `LOG.blobs/<sha256>`.

use serde_json::{json, Value as Json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const BLOB_LIMIT: usize = 64 * 1024;

/// The log format this build writes and reads: the header's `wj_effect_log` field. Format 2 records the `--allow`
/// grants of the recording in `grants`, so a replay re-applies them without flags (ADR-0035).
pub const LOG_VERSION: u64 = 2;

/// Everything that determines a run besides the program text and the effects it asked for.
#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub program: String,
    pub compiler: String,
    pub sched_seed: u64,
    pub sim_seed: Option<u64>,
    pub hash_seed: u64,
    pub rng_seed: u64,
    pub deterministic_time: bool,
    pub args: Vec<String>,
    /// The `--allow EFFECT[=RESOURCE]` grants the recorded run had (`--allow-network` as its two grants).
    pub grants: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Event {
    pub seq: u64,
    pub task: u64,
    pub effect: String,
    pub op: String,
    pub args: Json,
    pub result: Json,
    pub vt_ns: u64,
}

/// Canonical text of a JSON value: object keys sorted at every level, no whitespace.
pub fn canon(v: &Json) -> String {
    let mut out = Vec::with_capacity(64);
    write_canon(v, &mut out);
    // serde_json writes valid UTF-8 and the structure around it is ASCII
    String::from_utf8(out).unwrap_or_default()
}

fn write_canon(v: &Json, out: &mut Vec<u8>) {
    match v {
        Json::Object(m) => {
            // `serde_json::Map` is a `BTreeMap` here (no `preserve_order`): iteration is already in key order
            out.push(b'{');
            for (i, (k, x)) in m.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                let _ = serde_json::to_writer(&mut *out, k);
                out.push(b':');
                write_canon(x, out);
            }
            out.push(b'}');
        }
        Json::Array(a) => {
            out.push(b'[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_canon(x, out);
            }
            out.push(b']');
        }
        other => {
            let _ = serde_json::to_writer(&mut *out, other);
        }
    }
}

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn header_line(h: &Header) -> String {
    canon(&json!({
        "wj_effect_log": LOG_VERSION,
        "program": h.program,
        "compiler": h.compiler,
        "sched_seed": h.sched_seed,
        "sim_seed": h.sim_seed,
        "hash_seed": h.hash_seed,
        "rng_seed": h.rng_seed,
        "det_time": h.deterministic_time,
        "args": h.args,
        "grants": h.grants,
    }))
}

pub fn blob_dir(log: &Path) -> PathBuf {
    let mut p = log.as_os_str().to_owned();
    p.push(".blobs");
    PathBuf::from(p)
}

/// The event as one line; a result over `BLOB_LIMIT` is stored in the blob directory and replaced by its hash.
pub fn event_line(e: &Event, log: &Path) -> std::io::Result<String> {
    let text = canon(&e.result);
    let result = if text.len() > BLOB_LIMIT {
        let sha = digest(text.as_bytes());
        let dir = blob_dir(log);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(&sha), text)?;
        format!("{{\"sha256\":\"{sha}\"}}")
    } else {
        text
    };
    // the keys in canonical (sorted) order: args, effect, op, result, seq, task, vt_ns
    Ok(format!(
        "{{\"args\":{},\"effect\":{},\"op\":{},\"result\":{result},\"seq\":{},\"task\":{},\"vt_ns\":{}}}",
        canon(&e.args),
        Json::String(e.effect.clone()),
        Json::String(e.op.clone()),
        e.seq,
        e.task,
        e.vt_ns
    ))
}

/// The header of the log at `path` (the first line), refusing a log of another format.
pub fn read_header(path: &Path) -> Result<Header, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read effect log {}: {e}", path.display()))?;
    parse_header(path, text.lines().next())
}

fn parse_header(path: &Path, first: Option<&str>) -> Result<Header, String> {
    let head: Json = serde_json::from_str(first.ok_or("the effect log is empty")?).map_err(|e| format!("bad log header: {e}"))?;
    match head["wj_effect_log"].as_u64() {
        Some(LOG_VERSION) => {}
        Some(old) if old < LOG_VERSION => {
            return Err(format!(
                "{} is an effect log of format {old}, written before the header recorded capability grants; this wj reads format {LOG_VERSION}. Record the run again with `wj run --record`",
                path.display()
            ))
        }
        Some(newer) => return Err(format!("{} is an effect log of format {newer}, newer than this wj reads (format {LOG_VERSION}); update wj", path.display())),
        None => return Err(format!("{} is not an effect log (its first line has no `wj_effect_log` version)", path.display())),
    }
    let strings = |k: &str| -> Vec<String> { head[k].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default() };
    Ok(Header {
        program: head["program"].as_str().unwrap_or("").to_string(),
        compiler: head["compiler"].as_str().unwrap_or("").to_string(),
        sched_seed: head["sched_seed"].as_u64().unwrap_or(0),
        sim_seed: head["sim_seed"].as_u64(),
        hash_seed: head["hash_seed"].as_u64().unwrap_or(0),
        rng_seed: head["rng_seed"].as_u64().unwrap_or(0),
        deterministic_time: head["det_time"].as_bool().unwrap_or(false),
        args: strings("args"),
        grants: strings("grants"),
    })
}

pub fn read_log(path: &Path) -> Result<(Header, Vec<Event>), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read effect log {}: {e}", path.display()))?;
    let mut lines = text.lines();
    let header = parse_header(path, lines.next())?;
    let mut events = Vec::new();
    for (n, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let j: Json = serde_json::from_str(line).map_err(|e| format!("bad event on log line {}: {e}", n + 2))?;
        let mut result = j["result"].clone();
        if let Some(sha) = result.get("sha256").and_then(|s| s.as_str()) {
            let blob = blob_dir(path).join(sha);
            let t = std::fs::read_to_string(&blob).map_err(|e| format!("missing blob {}: {e}", blob.display()))?;
            result = serde_json::from_str(&t).map_err(|e| format!("bad blob {}: {e}", blob.display()))?;
        }
        events.push(Event {
            seq: j["seq"].as_u64().unwrap_or(n as u64),
            task: j["task"].as_u64().unwrap_or(0),
            effect: j["effect"].as_str().unwrap_or("").to_string(),
            op: j["op"].as_str().unwrap_or("").to_string(),
            args: j["args"].clone(),
            result,
            vt_ns: j["vt_ns"].as_u64().unwrap_or(0),
        });
    }
    Ok((header, events))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first implementation (a String per key and leaf, keys sorted by hand): the oracle for the allocation-free one.
    fn old_canon(v: &Json, out: &mut String) {
        match v {
            Json::Object(m) => {
                let mut keys: Vec<&String> = m.keys().collect();
                keys.sort();
                out.push('{');
                for (i, k) in keys.into_iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push_str(&Json::String(k.clone()).to_string());
                    out.push(':');
                    old_canon(&m[k], out);
                }
                out.push('}');
            }
            Json::Array(a) => {
                out.push('[');
                for (i, x) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    old_canon(x, out);
                }
                out.push(']');
            }
            other => out.push_str(&other.to_string()),
        }
    }

    fn old(v: &Json) -> String {
        let mut s = String::new();
        old_canon(v, &mut s);
        s
    }

    #[test]
    fn canonical_text_matches_the_first_implementation() {
        for v in [
            json!({"b": [1, {"y": 1, "x": 2}], "a": null}),
            json!({"k\"ey\n": "caf\u{e9} \u{1f600} \t \u{1} \\", "z": [], "a": {}, "n": [-0.0, 1e300, 18446744073709551615u64, -9223372036854775808i64]}),
            json!("plain"),
            json!([null, true, 1.5, {"é": {"b": 1, "a": [2, 3]}}]),
        ] {
            assert_eq!(canon(&v), old(&v), "{v}");
        }
    }

    #[test]
    fn an_event_line_is_the_canonical_text_of_its_object() {
        let e = Event { seq: 7, task: 2, effect: "fs.read".into(), op: "fs::read \"x\"".into(), args: json!({"b": [1, {"z": null, "a": "\u{e9}\n"}], "a": 1.5}), result: json!({"ok": true, "n": -3}), vt_ns: 42 };
        let want = old(&json!({ "seq": e.seq, "task": e.task, "effect": e.effect, "op": e.op, "args": e.args, "result": e.result, "vt_ns": e.vt_ns }));
        assert_eq!(event_line(&e, Path::new("/tmp/none.wjlog")).unwrap(), want);
    }

    #[test]
    fn a_header_round_trips_with_its_grants_and_an_old_one_is_refused() {
        let h = Header {
            program: "p".into(),
            compiler: "c".into(),
            sched_seed: 3,
            sim_seed: None,
            hash_seed: 4,
            rng_seed: 5,
            deterministic_time: false,
            args: vec!["a".into()],
            grants: vec!["fs.read".into(), "env.read=HOME".into()],
        };
        let path = Path::new("x.wjlog");
        assert_eq!(parse_header(path, Some(&header_line(&h))).unwrap(), h);
        let old = r#"{"args":[],"compiler":"0.50.0","det_time":false,"hash_seed":0,"program":"x","rng_seed":1,"sched_seed":0,"sim_seed":null,"wj_effect_log":1}"#;
        let e = parse_header(path, Some(old)).unwrap_err();
        assert!(e.contains("format 1") && e.contains("wj run --record"), "{e}");
        assert!(parse_header(path, Some(r#"{"a":1}"#)).unwrap_err().contains("not an effect log"));
    }
}
