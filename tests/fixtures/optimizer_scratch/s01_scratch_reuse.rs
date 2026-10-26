// Per-iteration scratch buffers (String and Vec) that are only borrowed: scratch_reuse hoists them.
use std::fmt::Write;
fn main() {
    let seed: u64 = std::env::args().nth(1).unwrap().parse().unwrap();
    let mut total: u64 = 0;
    let mut out: Vec<String> = Vec::new();
    for i in 0..(5 + seed) {
        let mut line = String::new();
        for j in 0..(i % 4 + 1) {
            write!(line, "{}-", (i * j + seed) % 10).unwrap();
        }
        line.push_str("x");
        total += line.len() as u64;
        let mut parts = Vec::new();
        for k in 0..(i % 3 + 2) {
            parts.push((k * seed + i) % 7);
        }
        parts.sort();
        total += parts.iter().sum::<u64>() * parts[0];
        out.push(line.clone());
    }
    // A buffer that is moved out each iteration must NOT be hoisted.
    let mut kept: Vec<String> = Vec::new();
    for i in 0..3 {
        let mut s = String::new();
        s.push_str(&(i + seed).to_string());
        kept.push(s);
    }
    println!("{} {} {}", total, out.join(","), kept.join("|"));
}
