use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::io::Read;

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut heap: BinaryHeap<Reverse<(i64, u64, String)>> = BinaryHeap::new();
    let mut seq = 0u64;
    for line in s.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.is_empty() {
            continue;
        }
        match p[0] {
            "push" => {
                heap.push(Reverse((p[1].parse().unwrap(), seq, p[2].to_string())));
                seq += 1;
            }
            "pop" => match heap.pop() {
                Some(Reverse((pr, _, l))) => println!("{} {}", pr, l),
                None => println!("empty"),
            },
            "peek" => match heap.peek() {
                Some(Reverse((pr, _, l))) => println!("{} {}", pr, l),
                None => println!("empty"),
            },
            "size" => println!("{}", heap.len()),
            _ => {}
        }
    }
}
