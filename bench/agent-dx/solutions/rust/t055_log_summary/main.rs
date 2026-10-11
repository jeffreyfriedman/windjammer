use std::collections::BTreeMap;
use std::io::Read;

const LEVELS: [&str; 4] = ["DEBUG", "INFO", "WARN", "ERROR"];

fn valid_ts(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 19
        && b.iter().enumerate().all(|(i, &c)| match i {
            4 | 7 => c == b'-',
            10 => c == b'T',
            13 | 16 => c == b':',
            _ => c.is_ascii_digit(),
        })
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut levels = [0u32; 4];
    let mut comps: BTreeMap<String, u32> = BTreeMap::new();
    let (mut first, mut last): (Option<String>, Option<String>) = (None, None);
    let mut errors: Vec<String> = Vec::new();
    let mut malformed = 0;
    for line in s.lines() {
        let p: Vec<&str> = line.splitn(4, ' ').collect();
        let lv = if p.len() >= 3 { LEVELS.iter().position(|l| *l == p[1]) } else { None };
        match lv {
            Some(li) if valid_ts(p[0]) && !p[2].is_empty() => {
                levels[li] += 1;
                *comps.entry(p[2].to_string()).or_insert(0) += 1;
                if first.as_deref().map_or(true, |f| p[0] < f) {
                    first = Some(p[0].to_string());
                }
                if last.as_deref().map_or(true, |l| p[0] > l) {
                    last = Some(p[0].to_string());
                }
                if li == 3 {
                    errors.push(format!("  [{}] {}", p[2], p.get(3).unwrap_or(&"")));
                }
            }
            _ => malformed += 1,
        }
    }
    println!("levels: DEBUG={} INFO={} WARN={} ERROR={}", levels[0], levels[1], levels[2], levels[3]);
    println!("top components:");
    let mut cv: Vec<(&String, &u32)> = comps.iter().collect();
    cv.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (c, n) in cv.iter().take(3) {
        println!("  {} {}", c, n);
    }
    println!("first: {}", first.as_deref().unwrap_or("-"));
    println!("last: {}", last.as_deref().unwrap_or("-"));
    println!("errors:");
    for e in errors {
        println!("{}", e);
    }
    println!("malformed: {}", malformed);
}
