use std::collections::HashMap;
use std::io::Read;

fn parse_int(s: &str, lo: u64, hi: u64) -> Option<u64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let t = s.trim_start_matches('0');
    if t.len() > 18 {
        return None;
    }
    let v: u64 = if t.is_empty() { 0 } else { t.parse().ok()? };
    if v >= lo && v <= hi {
        Some(v)
    } else {
        None
    }
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut errors: Vec<String> = Vec::new();
    let mut map: HashMap<String, String> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for (i, line) in s.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        match line.split_once('=') {
            None => errors.push(format!("error: malformed line {}", i + 1)),
            Some((k, v)) => {
                if !map.contains_key(k) {
                    order.push(k.to_string());
                }
                map.insert(k.to_string(), v.to_string());
            }
        }
    }
    let mut host = String::new();
    let (mut port, mut workers, mut debug) = (0u64, 4u64, false);
    match map.get("host") {
        None => errors.push("error: host: missing".into()),
        Some(h) if h.is_empty() => errors.push("error: host: empty".into()),
        Some(h) => host = h.clone(),
    }
    match map.get("port") {
        None => errors.push("error: port: missing".into()),
        Some(v) => match parse_int(v, 1, 65535) {
            Some(p) => port = p,
            None => errors.push(format!("error: port: invalid value {}", v)),
        },
    }
    if let Some(v) = map.get("debug") {
        match v.as_str() {
            "true" => debug = true,
            "false" => debug = false,
            _ => errors.push(format!("error: debug: invalid value {}", v)),
        }
    }
    if let Some(v) = map.get("workers") {
        match parse_int(v, 1, 64) {
            Some(w) => workers = w,
            None => errors.push(format!("error: workers: invalid value {}", v)),
        }
    }
    for k in &order {
        if !["host", "port", "debug", "workers"].contains(&k.as_str()) {
            errors.push(format!("error: unknown key {}", k));
        }
    }
    if errors.is_empty() {
        println!("ok host={} port={} debug={} workers={}", host, port, debug, workers);
    } else {
        // malformed lines already first; schema errors follow in order
        for e in errors {
            println!("{}", e);
        }
    }
}
