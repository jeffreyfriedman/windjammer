use std::collections::VecDeque;
use std::io::Read;

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut l: VecDeque<i64> = VecDeque::new();
    for line in s.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.is_empty() {
            continue;
        }
        match p[0] {
            "push_front" => l.push_front(p[1].parse().unwrap()),
            "push_back" => l.push_back(p[1].parse().unwrap()),
            "pop_front" | "pop_back" => {
                let v = if p[0] == "pop_front" { l.pop_front() } else { l.pop_back() };
                match v {
                    Some(x) => println!("{}", x),
                    None => println!("empty"),
                }
            }
            "remove" => {
                let x: i64 = p[1].parse().unwrap();
                match l.iter().position(|&e| e == x) {
                    Some(i) => {
                        l.remove(i);
                        println!("removed");
                    }
                    None => println!("missing"),
                }
            }
            "reverse" => l.make_contiguous().reverse(),
            "print" => {
                if l.is_empty() {
                    println!("<empty>");
                } else {
                    let v: Vec<String> = l.iter().map(|x| x.to_string()).collect();
                    println!("{}", v.join(" "));
                }
            }
            "nth" => {
                let k: i64 = p[1].parse().unwrap();
                match if k < 0 { None } else { l.get(k as usize) } {
                    Some(x) => println!("{}", x),
                    None => println!("none"),
                }
            }
            _ => {}
        }
    }
}
