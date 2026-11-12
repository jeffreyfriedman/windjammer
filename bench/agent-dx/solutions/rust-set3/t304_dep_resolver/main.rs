use std::collections::{BTreeMap, HashMap};
use std::io::{self, Read};

#[derive(Clone, Copy, PartialEq)]
enum Op {
    Ge,
    Le,
    Eq,
    Gt,
    Lt,
}

impl Op {
    fn parse(s: &str) -> Option<Op> {
        Some(match s {
            ">=" => Op::Ge,
            "<=" => Op::Le,
            "=" => Op::Eq,
            ">" => Op::Gt,
            "<" => Op::Lt,
            _ => return None,
        })
    }
    fn text(self) -> &'static str {
        match self {
            Op::Ge => ">=",
            Op::Le => "<=",
            Op::Eq => "=",
            Op::Gt => ">",
            Op::Lt => "<",
        }
    }
    fn holds(self, have: u32, want: u32) -> bool {
        match self {
            Op::Ge => have >= want,
            Op::Le => have <= want,
            Op::Eq => have == want,
            Op::Gt => have > want,
            Op::Lt => have < want,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    InProgress,
    Done,
}

struct Resolver<'a> {
    versions: &'a HashMap<String, u32>,
    // package -> dependency -> constraints in input order
    deps: &'a BTreeMap<String, BTreeMap<String, Vec<(Op, u32)>>>,
    state: HashMap<&'a str, State>,
    chain: Vec<&'a str>,
    order: Vec<&'a str>,
    errors: &'a mut Vec<String>,
}

impl<'a> Resolver<'a> {
    fn visit(&mut self, u: &'a str) {
        self.state.insert(u, State::InProgress);
        self.chain.push(u);
        if let Some(ds) = self.deps.get(u) {
            for (d, constraints) in ds {
                let have = match self.versions.get(d.as_str()) {
                    Some(&v) => v,
                    None => {
                        self.errors.push(format!("error: missing dependency {} of {}", d, u));
                        continue;
                    }
                };
                for &(op, want) in constraints {
                    if !op.holds(have, want) {
                        self.errors.push(format!("error: {} needs {} {} {} but found {}", u, d, op.text(), want, have));
                    }
                }
                match self.state.get(d.as_str()).copied() {
                    Some(State::InProgress) => {
                        let at = self.chain.iter().position(|&x| x == d.as_str()).unwrap();
                        let mut cyc: Vec<&str> = self.chain[at..].to_vec();
                        cyc.push(d.as_str());
                        self.errors.push(format!("error: cycle {}", cyc.join(" -> ")));
                    }
                    Some(State::Done) => {}
                    None => self.visit(d.as_str()),
                }
            }
        }
        self.chain.pop();
        self.state.insert(u, State::Done);
        self.order.push(u);
    }
}

fn number(s: &str) -> Option<u32> {
    if s.is_empty() || s.len() > 6 || !s.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut errors: Vec<String> = Vec::new();
    let mut versions: HashMap<String, u32> = HashMap::new();
    let mut dep_lines: Vec<(String, String, Op, u32)> = Vec::new();
    let mut targets: Vec<String> = Vec::new();
    for (i, line) in input.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let p: Vec<&str> = line.split(' ').collect();
        let well_formed = p.iter().all(|t| !t.is_empty());
        let mut ok = false;
        if well_formed {
            match (p[0], p.len()) {
                ("pkg", 3) => {
                    if let Some(v) = number(p[2]) {
                        ok = true;
                        if versions.contains_key(p[1]) {
                            errors.push(format!("error: duplicate package {}", p[1]));
                        } else {
                            versions.insert(p[1].to_string(), v);
                        }
                    }
                }
                ("dep", 5) => {
                    if let (Some(op), Some(v)) = (Op::parse(p[3]), number(p[4])) {
                        dep_lines.push((p[1].to_string(), p[2].to_string(), op, v));
                        ok = true;
                    }
                }
                ("install", 2) => {
                    targets.push(p[1].to_string());
                    ok = true;
                }
                _ => {}
            }
        }
        if !ok {
            errors.push(format!("error: bad line {}", i + 1));
        }
    }
    let mut deps: BTreeMap<String, BTreeMap<String, Vec<(Op, u32)>>> = BTreeMap::new();
    for (name, dep, op, v) in dep_lines {
        if !versions.contains_key(&name) {
            errors.push(format!("error: dep for unknown package {}", name));
            continue;
        }
        deps.entry(name).or_default().entry(dep).or_default().push((op, v));
    }
    let order: Vec<String> = {
        let mut r = Resolver {
            versions: &versions,
            deps: &deps,
            state: HashMap::new(),
            chain: Vec::new(),
            order: Vec::new(),
            errors: &mut errors,
        };
        for t in &targets {
            match versions.get_key_value(t.as_str()) {
                None => r.errors.push(format!("error: unknown package {}", t)),
                Some((name, _)) => {
                    if !r.state.contains_key(name.as_str()) {
                        r.visit(name.as_str());
                    }
                }
            }
        }
        r.order.iter().map(|s| s.to_string()).collect()
    };
    if !errors.is_empty() {
        for e in &errors {
            println!("{}", e);
        }
        std::process::exit(1);
    }
    for n in order {
        println!("install {} {}", n, versions[&n]);
    }
}
