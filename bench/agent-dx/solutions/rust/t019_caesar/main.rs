use std::io::Read;

fn shift(c: char, n: i64) -> char {
    let base = if c.is_ascii_lowercase() {
        b'a'
    } else if c.is_ascii_uppercase() {
        b'A'
    } else {
        return c;
    };
    let off = (c as u8 - base) as i64;
    (base + (off + n).rem_euclid(26) as u8) as char
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let (head, rest) = match s.find('\n') {
        Some(i) => (&s[..i], &s[i + 1..]),
        None => (&s[..], ""),
    };
    let mut p = head.split_whitespace();
    let mode = p.next().unwrap();
    let n: i64 = p.next().unwrap().parse().unwrap();
    let n = if mode == "dec" { -n } else { n };
    let out: String = rest.chars().map(|c| shift(c, n)).collect();
    print!("{}", out);
}
