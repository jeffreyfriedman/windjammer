use std::io::Read;
use std::sync::mpsc::{channel, Receiver};
use std::thread;

#[derive(Clone, Copy)]
enum Stage {
    Add(i64),
    Mul(i64),
    Gt(i64),
    Even,
    Odd,
    Dedupe3,
}

fn parse_stage(tok: &str) -> Option<Stage> {
    let (name, arg) = match tok.split_once(':') {
        Some((n, a)) => (n, a.parse::<i64>().ok()),
        None => (tok, None),
    };
    match (name, arg) {
        ("add", Some(k)) => Some(Stage::Add(k)),
        ("mul", Some(k)) => Some(Stage::Mul(k)),
        ("gt", Some(k)) => Some(Stage::Gt(k)),
        ("even", _) => Some(Stage::Even),
        ("odd", _) => Some(Stage::Odd),
        ("dedupe3", _) => Some(Stage::Dedupe3),
        _ => None,
    }
}

fn spawn_stage(stage: Stage, input: Receiver<i64>) -> Receiver<i64> {
    let (tx, rx) = channel();
    thread::spawn(move || {
        let mut prev: Vec<i64> = Vec::new();
        for x in input {
            let out = match stage {
                Stage::Add(k) => Some(x.wrapping_add(k)),
                Stage::Mul(k) => Some(x.wrapping_mul(k)),
                Stage::Gt(k) => if x > k { Some(x) } else { None },
                Stage::Even => if x % 2 == 0 { Some(x) } else { None },
                Stage::Odd => if x % 2 != 0 { Some(x) } else { None },
                Stage::Dedupe3 => {
                    let dup = prev.contains(&x);
                    prev.push(x);
                    if prev.len() > 2 {
                        prev.remove(0);
                    }
                    if dup { None } else { Some(x) }
                }
            };
            if let Some(v) = out {
                if tx.send(v).is_err() {
                    break;
                }
            }
        }
    });
    rx
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut lines = s.lines();
    let n: i64 = lines.next().unwrap().trim().parse().unwrap();
    let stages: Vec<Stage> = lines
        .next()
        .unwrap_or("")
        .split_whitespace()
        .filter_map(parse_stage)
        .collect();
    let (tx, rx) = channel();
    thread::spawn(move || {
        for i in 1..=n {
            if tx.send(i).is_err() {
                break;
            }
        }
    });
    let mut cur = rx;
    for st in stages {
        cur = spawn_stage(st, cur);
    }
    let items: Vec<i64> = cur.iter().collect();
    println!("{}", items.len());
    println!("{}", items.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" "));
    println!("sum={}", items.iter().fold(0i64, |a, &b| a.wrapping_add(b)));
}
