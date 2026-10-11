use std::collections::HashMap;
use std::io::Read;

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut data: HashMap<(String, String), String> = HashMap::new();
    let mut section = String::new();
    let mut lines = s.lines().map(|l| l.trim_end_matches('\r'));
    for line in lines.by_ref() {
        if line == "---" {
            break;
        }
        let t = line.trim();
        if t.is_empty() || t.starts_with(';') || t.starts_with('#') {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            section = t[1..t.len() - 1].to_string();
        } else if let Some((k, v)) = t.split_once('=') {
            data.insert((section.clone(), k.trim().to_string()), v.trim().to_string());
        }
    }
    for q in lines {
        let (sec, key) = q.split_once('.').unwrap_or(("", q));
        match data.get(&(sec.to_string(), key.to_string())) {
            Some(v) => println!("{}", v),
            None => println!("<none>"),
        }
    }
}
