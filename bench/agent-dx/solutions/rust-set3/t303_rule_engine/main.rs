use std::io::{self, Read};

struct Line {
    sku: String,
    qty: i64,
    base: i64,
    unit: i64,
    cur: i64,
}

trait Rule {
    fn kind(&self) -> &'static str;
    // returns the saving; may change lines and/or the cart-level discount
    fn apply(&self, lines: &mut [Line], cart: &mut i64) -> i64;
}

fn matches(target: &str, l: &Line) -> bool {
    target == "*" || target == l.sku
}

struct Percent {
    target: String,
    p: i64,
}
struct Fixed {
    target: String,
    c: i64,
}
struct Bogo {
    target: String,
    n: i64,
    m: i64,
}
struct Threshold {
    t: i64,
    p: i64,
}

impl Rule for Percent {
    fn kind(&self) -> &'static str {
        "percent"
    }
    fn apply(&self, lines: &mut [Line], _cart: &mut i64) -> i64 {
        lines
            .iter_mut()
            .filter(|l| matches(&self.target, l))
            .map(|l| {
                let d = l.cur * self.p / 100;
                l.cur -= d;
                d
            })
            .sum()
    }
}
impl Rule for Fixed {
    fn kind(&self) -> &'static str {
        "fixed"
    }
    fn apply(&self, lines: &mut [Line], _cart: &mut i64) -> i64 {
        let mut saved = 0;
        for l in lines.iter_mut().filter(|l| matches(&self.target, l)) {
            let d = l.cur.min(self.c);
            l.cur -= d;
            saved += d;
        }
        saved
    }
}
impl Rule for Bogo {
    fn kind(&self) -> &'static str {
        "bogo"
    }
    fn apply(&self, lines: &mut [Line], _cart: &mut i64) -> i64 {
        let mut saved = 0;
        for l in lines.iter_mut() {
            if matches(&self.target, l) {
                let free = l.qty / (self.n + self.m) * self.m;
                let d = l.cur.min(free * l.unit);
                l.cur -= d;
                saved += d;
            }
        }
        saved
    }
}
impl Rule for Threshold {
    fn kind(&self) -> &'static str {
        "threshold"
    }
    fn apply(&self, lines: &mut [Line], cart: &mut i64) -> i64 {
        let running: i64 = lines.iter().map(|l| l.cur).sum::<i64>() - *cart;
        if running >= self.t {
            let d = running * self.p / 100;
            *cart += d;
            d
        } else {
            0
        }
    }
}

fn num(s: &str, lo: i64, hi: i64) -> Option<i64> {
    if s.is_empty() || s.len() > 9 || !s.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let v: i64 = s.parse().ok()?;
    if v >= lo && v <= hi {
        Some(v)
    } else {
        None
    }
}

const MAXV: i64 = 999_999_999;

fn parse_rule(p: &[&str]) -> Option<Box<dyn Rule>> {
    if p.len() < 2 || p[0] != "rule" {
        return None;
    }
    match (p[1], p.len()) {
        ("percent", 4) if !p[2].is_empty() => Some(Box::new(Percent { target: p[2].to_string(), p: num(p[3], 0, 100)? })),
        ("fixed", 4) if !p[2].is_empty() => Some(Box::new(Fixed { target: p[2].to_string(), c: num(p[3], 0, MAXV)? })),
        ("bogo", 5) if !p[2].is_empty() => Some(Box::new(Bogo {
            target: p[2].to_string(),
            n: num(p[3], 1, MAXV)?,
            m: num(p[4], 1, MAXV)?,
        })),
        ("threshold", 4) => Some(Box::new(Threshold { t: num(p[2], 0, MAXV)?, p: num(p[3], 0, 100)? })),
        _ => None,
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut errors: Vec<String> = Vec::new();
    let mut lines: Vec<Line> = Vec::new();
    let mut rules: Vec<Box<dyn Rule>> = Vec::new();
    for (i, raw) in input.lines().enumerate() {
        if raw.is_empty() {
            continue;
        }
        let p: Vec<&str> = raw.split(' ').collect();
        let mut ok = false;
        if p[0] == "item" {
            if p.len() == 4 && !p[1].is_empty() {
                if let (Some(q), Some(u)) = (num(p[2], 1, MAXV), num(p[3], 0, MAXV)) {
                    lines.push(Line { sku: p[1].to_string(), qty: q, unit: u, base: q * u, cur: q * u });
                    ok = true;
                }
            }
        } else if let Some(r) = parse_rule(&p) {
            rules.push(r);
            ok = true;
        }
        if !ok {
            errors.push(format!("error line {}: {}", i + 1, raw));
        }
    }
    let mut cart = 0i64;
    let mut rule_out = Vec::new();
    for (i, r) in rules.iter().enumerate() {
        let saved = r.apply(&mut lines, &mut cart);
        rule_out.push(format!("rule {} {} {}", i + 1, r.kind(), saved));
    }
    for e in &errors {
        println!("{}", e);
    }
    for l in &lines {
        println!("item {} {} {}", l.sku, l.base, l.cur);
    }
    for r in &rule_out {
        println!("{}", r);
    }
    let total = (lines.iter().map(|l| l.cur).sum::<i64>() - cart).max(0);
    println!("total {}", total);
    std::process::exit(if errors.is_empty() { 0 } else { 1 });
}
