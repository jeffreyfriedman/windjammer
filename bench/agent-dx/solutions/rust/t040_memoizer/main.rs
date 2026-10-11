use std::collections::HashMap;
use std::io::Read;

struct Memo<F: Fn(u64) -> u64> {
    f: F,
    cache: HashMap<u64, u64>,
    hits: u32,
    misses: u32,
}

impl<F: Fn(u64) -> u64> Memo<F> {
    fn new(f: F) -> Self {
        Memo { f, cache: HashMap::new(), hits: 0, misses: 0 }
    }
    fn get(&mut self, arg: u64) -> (u64, bool) {
        if let Some(&v) = self.cache.get(&arg) {
            self.hits += 1;
            return (v, true);
        }
        let v = (self.f)(arg);
        self.cache.insert(arg, v);
        self.misses += 1;
        (v, false)
    }
}

fn fib(n: u64) -> u64 {
    let (mut a, mut b) = (0u64, 1u64);
    for _ in 0..n {
        let t = (a + b) % 1_000_000_007;
        a = b;
        b = t;
    }
    a
}

fn collatz(mut n: u64) -> u64 {
    let mut c = 0;
    while n > 1 {
        n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        c += 1;
    }
    c
}

fn tri(n: u64) -> u64 {
    n * (n + 1) / 2
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut mf = Memo::new(fib);
    let mut mc = Memo::new(collatz);
    let mut mt = Memo::new(tri);
    let (mut h, mut m) = (0, 0);
    for line in s.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.len() < 2 {
            continue;
        }
        let a: u64 = p[1].parse().unwrap();
        let r = match p[0] {
            "fib" => mf.get(a),
            "collatz" => mc.get(a),
            "tri" => mt.get(a),
            _ => {
                println!("unknown function");
                continue;
            }
        };
        if r.1 { h += 1 } else { m += 1 }
        println!("{} {}", r.0, if r.1 { "hit" } else { "miss" });
    }
    println!("hits={} misses={}", h, m);
}
