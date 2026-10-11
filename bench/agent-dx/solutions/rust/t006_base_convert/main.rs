use std::io::Read;

fn digit(c: char) -> u64 {
    c.to_digit(36).unwrap() as u64
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    for line in s.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.len() < 3 {
            continue;
        }
        let from: u64 = p[1].parse().unwrap();
        let to: u64 = p[2].parse().unwrap();
        let mut v: u64 = 0;
        for c in p[0].chars() {
            v = v * from + digit(c);
        }
        if v == 0 {
            println!("0");
            continue;
        }
        let mut out = Vec::new();
        while v > 0 {
            out.push(std::char::from_digit((v % to) as u32, 36).unwrap().to_ascii_uppercase());
            v /= to;
        }
        let r: String = out.iter().rev().collect();
        println!("{}", r);
    }
}
