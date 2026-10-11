use std::io::Read;

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();
    let t: usize = it.next().unwrap().parse().unwrap();
    for _ in 0..t {
        let tok = it.next().unwrap();
        let n: u64 = tok.parse().unwrap();
        let text = n.to_string();
        let sum: u64 = text.bytes().map(|b| (b - b'0') as u64).sum();
        let root = if sum == 0 { 0 } else { 1 + (sum - 1) % 9 };
        let rev: String = text.chars().rev().collect();
        let rev_n: u64 = rev.parse().unwrap();
        let pal = if rev == text { "yes" } else { "no" };
        println!("{} {} {} {}", sum, root, rev_n, pal);
    }
}
