use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};
use std::io::{self, Read};

#[derive(Default)]
struct Graph {
    out: BTreeMap<String, BTreeMap<String, u64>>,
    inc: BTreeMap<String, BTreeMap<String, u64>>,
}

impl Graph {
    fn add(&mut self, a: &str, b: &str, w: u64) {
        self.out.entry(b.to_string()).or_default();
        self.inc.entry(a.to_string()).or_default();
        let e = self.out.entry(a.to_string()).or_default().entry(b.to_string()).or_insert(w);
        if w < *e {
            *e = w;
        }
        let e = self.inc.entry(b.to_string()).or_default().entry(a.to_string()).or_insert(w);
        if w < *e {
            *e = w;
        }
    }

    fn known(&self, n: &str) -> bool {
        self.out.contains_key(n)
    }

    // distance of every node to `target`, following edges backwards
    fn dist_to<'a>(&'a self, target: &'a str) -> BTreeMap<&'a str, u64> {
        let mut dist: BTreeMap<&str, u64> = BTreeMap::new();
        let mut heap = BinaryHeap::new();
        dist.insert(target, 0);
        heap.push(Reverse((0u64, target)));
        while let Some(Reverse((d, u))) = heap.pop() {
            if dist.get(u).map_or(false, |&best| d > best) {
                continue;
            }
            if let Some(preds) = self.inc.get(u) {
                for (p, w) in preds {
                    let nd = d + w;
                    let better = match dist.get(p.as_str()) {
                        Some(&cur) => nd < cur,
                        None => true,
                    };
                    if better {
                        dist.insert(p.as_str(), nd);
                        heap.push(Reverse((nd, p.as_str())));
                    }
                }
            }
        }
        dist
    }

    fn route(&self, a: &str, b: &str) -> Option<(u64, Vec<String>)> {
        let dist = self.dist_to(b);
        let total = *dist.get(a)?;
        let mut path = vec![a.to_string()];
        let mut cur: &str = a;
        while cur != b {
            let mut next: Option<&str> = None;
            // BTreeMap iterates in byte order, so the first match is the smallest name
            for (v, w) in &self.out[cur] {
                if let Some(&dv) = dist.get(v.as_str()) {
                    if w + dv == dist[cur] {
                        next = Some(v.as_str());
                        break;
                    }
                }
            }
            let n = next?;
            path.push(n.to_string());
            cur = n;
        }
        Some((total, path))
    }
}

fn valid_weight(s: &str) -> Option<u64> {
    if s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()) || s.len() > 12 {
        return None;
    }
    let v: u64 = s.parse().ok()?;
    if (1..=1_000_000).contains(&v) {
        Some(v)
    } else {
        None
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut g = Graph::default();
    let mut out = String::new();
    for line in input.lines() {
        if line.is_empty() {
            continue;
        }
        let p: Vec<&str> = line.split(' ').collect();
        let named = p.iter().all(|t| !t.is_empty());
        if named && p.len() == 4 && p[0] == "edge" && valid_weight(p[3]).is_some() {
            g.add(p[1], p[2], valid_weight(p[3]).unwrap());
        } else if named && p.len() == 3 && p[0] == "path" {
            let (a, b) = (p[1], p[2]);
            if !g.known(a) {
                out.push_str(&format!("unknown node {}\n", a));
            } else if !g.known(b) {
                out.push_str(&format!("unknown node {}\n", b));
            } else if a == b {
                out.push_str(&format!("0 {}\n", a));
            } else {
                match g.route(a, b) {
                    Some((c, path)) => out.push_str(&format!("{} {}\n", c, path.join(">"))),
                    None => out.push_str("unreachable\n"),
                }
            }
        } else {
            out.push_str(&format!("bad command: {}\n", line));
        }
    }
    print!("{}", out);
}
