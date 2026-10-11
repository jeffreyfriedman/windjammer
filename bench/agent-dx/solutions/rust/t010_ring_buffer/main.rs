use std::collections::VecDeque;
use std::io::Read;

struct Ring {
    cap: usize,
    items: VecDeque<i64>,
}

impl Ring {
    fn push(&mut self, x: i64) -> &'static str {
        if self.items.len() >= self.cap {
            "full"
        } else {
            self.items.push_back(x);
            "ok"
        }
    }
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut lines = s.lines();
    let cap: usize = lines.next().unwrap().trim().parse().unwrap();
    let mut r = Ring { cap, items: VecDeque::new() };
    for line in lines {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.is_empty() {
            continue;
        }
        match p[0] {
            "push" => println!("{}", r.push(p[1].parse().unwrap())),
            "pop" => match r.items.pop_front() {
                Some(v) => println!("{}", v),
                None => println!("empty"),
            },
            "peek" => match r.items.front() {
                Some(v) => println!("{}", v),
                None => println!("empty"),
            },
            "size" => println!("{}", r.items.len()),
            "dump" => {
                if r.items.is_empty() {
                    println!("<empty>");
                } else {
                    let v: Vec<String> = r.items.iter().map(|x| x.to_string()).collect();
                    println!("{}", v.join(" "));
                }
            }
            _ => {}
        }
    }
}
