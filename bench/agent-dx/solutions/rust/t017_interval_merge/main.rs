use std::io::Read;

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();
    let n: usize = match it.next() {
        Some(t) => t.parse().unwrap(),
        None => 0,
    };
    let mut v: Vec<(i64, i64)> = (0..n)
        .map(|_| {
            let a = it.next().unwrap().parse().unwrap();
            let b = it.next().unwrap().parse().unwrap();
            (a, b)
        })
        .collect();
    v.sort();
    let mut merged: Vec<(i64, i64)> = Vec::new();
    for (a, b) in v {
        match merged.last_mut() {
            Some(last) if a <= last.1 => last.1 = last.1.max(b),
            _ => merged.push((a, b)),
        }
    }
    for (a, b) in merged {
        println!("{} {}", a, b);
    }
}
