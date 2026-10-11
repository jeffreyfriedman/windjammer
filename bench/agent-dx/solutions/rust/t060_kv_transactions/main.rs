use std::collections::HashMap;
use std::io::Read;

#[derive(Default)]
struct Store {
    data: HashMap<String, i64>,
    undo: Vec<HashMap<String, Option<i64>>>,
}

impl Store {
    fn record(&mut self, k: &str) {
        let old = self.data.get(k).copied();
        if let Some(top) = self.undo.last_mut() {
            top.entry(k.to_string()).or_insert(old);
        }
    }
    fn set(&mut self, k: &str, v: i64) {
        self.record(k);
        self.data.insert(k.to_string(), v);
    }
    fn delete(&mut self, k: &str) {
        self.record(k);
        self.data.remove(k);
    }
    fn rollback(&mut self) -> bool {
        match self.undo.pop() {
            None => false,
            Some(log) => {
                for (k, old) in log {
                    match old {
                        Some(v) => self.data.insert(k, v),
                        None => self.data.remove(&k),
                    };
                }
                true
            }
        }
    }
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut st = Store::default();
    for line in s.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.is_empty() {
            continue;
        }
        match p[0] {
            "set" if p.len() >= 3 => st.set(p[1], p[2].parse().unwrap()),
            "get" if p.len() >= 2 => match st.data.get(p[1]) {
                Some(v) => println!("{}", v),
                None => println!("NULL"),
            },
            "delete" if p.len() >= 2 => st.delete(p[1]),
            "count" if p.len() >= 2 => {
                let v: i64 = p[1].parse().unwrap();
                println!("{}", st.data.values().filter(|&&x| x == v).count());
            }
            "begin" => st.undo.push(HashMap::new()),
            "rollback" => {
                if !st.rollback() {
                    println!("NO TRANSACTION");
                }
            }
            "commit" => {
                if st.undo.is_empty() {
                    println!("NO TRANSACTION");
                }
                st.undo.clear();
            }
            _ => {}
        }
    }
}
