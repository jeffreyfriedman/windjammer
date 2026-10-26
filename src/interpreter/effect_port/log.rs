//! Effect log v1 (ADR-0021 section 10): a JSON Lines file, one header then one event per line, canonical JSON
//! (sorted keys) so two logs of the same run diff cleanly. Payloads over `BLOB_LIMIT` bytes live in `LOG.blobs/<sha256>`.

use serde_json::{json, Value as Json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const BLOB_LIMIT: usize = 64 * 1024;

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
    let mut s = String::new();
    write_canon(v, &mut s);
    s
}

fn write_canon(v: &Json, out: &mut String) {
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
                write_canon(&m[k], out);
            }
            out.push('}');
        }
        Json::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canon(x, out);
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn header_line(h: &Header) -> String {
    canon(&json!({
        "wj_effect_log": 1,
        "program": h.program,
        "compiler": h.compiler,
        "sched_seed": h.sched_seed,
        "sim_seed": h.sim_seed,
        "hash_seed": h.hash_seed,
        "rng_seed": h.rng_seed,
        "det_time": h.deterministic_time,
        "args": h.args,
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
        json!({ "sha256": sha })
    } else {
        e.result.clone()
    };
    Ok(canon(&json!({
        "seq": e.seq, "task": e.task, "effect": e.effect, "op": e.op, "args": e.args, "result": result, "vt_ns": e.vt_ns,
    })))
}

pub fn read_log(path: &Path) -> Result<(Header, Vec<Event>), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read effect log {}: {e}", path.display()))?;
    let mut lines = text.lines();
    let head: Json = serde_json::from_str(lines.next().ok_or("the effect log is empty")?).map_err(|e| format!("bad log header: {e}"))?;
    if head["wj_effect_log"] != 1 {
        return Err(format!("{} is not an effect log of version 1", path.display()));
    }
    let header = Header {
        program: head["program"].as_str().unwrap_or("").to_string(),
        compiler: head["compiler"].as_str().unwrap_or("").to_string(),
        sched_seed: head["sched_seed"].as_u64().unwrap_or(0),
        sim_seed: head["sim_seed"].as_u64(),
        hash_seed: head["hash_seed"].as_u64().unwrap_or(0),
        rng_seed: head["rng_seed"].as_u64().unwrap_or(0),
        deterministic_time: head["det_time"].as_bool().unwrap_or(false),
        args: head["args"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default(),
    };
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

    #[test]
    fn canonical_json_sorts_keys_at_every_level() {
        let v = json!({"b": 1, "a": {"z": [1, {"y": 2, "x": 3}], "c": null}});
        assert_eq!(canon(&v), r#"{"a":{"c":null,"z":[1,{"x":3,"y":2}]},"b":1}"#);
    }

    #[test]
    fn a_log_round_trips_and_big_results_go_to_blobs() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("run.wjlog");
        let h = Header { program: "abc".into(), compiler: "0.50.0".into(), sched_seed: 3, sim_seed: None, hash_seed: 0, rng_seed: 9, deterministic_time: false, args: vec!["x".into()] };
        let big = "x".repeat(BLOB_LIMIT + 10);
        let events = [
            Event { seq: 0, task: 0, effect: "fs.read".into(), op: "fs::read_to_string".into(), args: json!(["a.txt"]), result: json!("hi"), vt_ns: 0 },
            Event { seq: 1, task: 1, effect: "fs.read".into(), op: "fs::read_to_string".into(), args: json!(["b.txt"]), result: json!(big), vt_ns: 5 },
        ];
        let mut text = header_line(&h) + "\n";
        for e in &events {
            text += &(event_line(e, &p).unwrap() + "\n");
        }
        std::fs::write(&p, text).unwrap();
        let (h2, e2) = read_log(&p).unwrap();
        assert_eq!(h2, h);
        assert_eq!(e2.len(), 2);
        assert_eq!(e2[1].result, events[1].result);
        assert!(blob_dir(&p).read_dir().unwrap().count() == 1);
    }
}
