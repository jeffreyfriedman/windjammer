use std::io::Read;

struct Lru {
    cap: usize,
    // front = most recently used
    entries: Vec<(i64, i64)>,
}

impl Lru {
    fn get(&mut self, k: i64) -> i64 {
        match self.entries.iter().position(|e| e.0 == k) {
            Some(i) => {
                let e = self.entries.remove(i);
                self.entries.insert(0, e);
                e.1
            }
            None => -1,
        }
    }

    fn put(&mut self, k: i64, v: i64) -> Option<i64> {
        if let Some(i) = self.entries.iter().position(|e| e.0 == k) {
            self.entries.remove(i);
        }
        self.entries.insert(0, (k, v));
        if self.entries.len() > self.cap {
            return self.entries.pop().map(|e| e.0);
        }
        None
    }
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut lines = s.lines();
    let cap: usize = lines.next().unwrap().trim().parse().unwrap();
    let mut c = Lru { cap, entries: Vec::new() };
    for line in lines {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.is_empty() {
            continue;
        }
        match p[0] {
            "put" => {
                if let Some(k) = c.put(p[1].parse().unwrap(), p[2].parse().unwrap()) {
                    println!("evict {}", k);
                }
            }
            "get" => println!("{}", c.get(p[1].parse().unwrap())),
            "keys" => {
                if c.entries.is_empty() {
                    println!("<empty>");
                } else {
                    let v: Vec<String> = c.entries.iter().map(|e| e.0.to_string()).collect();
                    println!("{}", v.join(" "));
                }
            }
            _ => {}
        }
    }
}
