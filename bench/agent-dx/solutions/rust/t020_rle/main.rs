use std::io::Read;

fn encode(t: &str) -> String {
    let mut out = String::new();
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        let mut n = 1;
        while chars.peek() == Some(&c) {
            chars.next();
            n += 1;
        }
        out.push_str(&format!("{}{}", n, c));
    }
    out
}

fn decode(t: &str) -> String {
    let mut out = String::new();
    let mut n: usize = 0;
    for c in t.chars() {
        if let Some(d) = c.to_digit(10) {
            n = n * 10 + d as usize;
        } else {
            out.push_str(&c.to_string().repeat(n));
            n = 0;
        }
    }
    out
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    for line in s.lines() {
        let line = line.trim_end_matches('\r');
        let (cmd, text) = match line.split_once(' ') {
            Some((c, t)) => (c, t),
            None => (line, ""),
        };
        match cmd {
            "E" => println!("{}", encode(text)),
            "D" => println!("{}", decode(text)),
            _ => {}
        }
    }
}
