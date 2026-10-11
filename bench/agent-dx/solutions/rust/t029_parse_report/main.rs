use std::io::Read;

enum Outcome {
    Ok(i64),
    Empty,
    Invalid,
    Overflow,
}

fn classify(tok: &str) -> Outcome {
    if tok.is_empty() {
        return Outcome::Empty;
    }
    let (neg, digits) = match tok.as_bytes()[0] {
        b'-' => (true, &tok[1..]),
        b'+' => (false, &tok[1..]),
        _ => (false, tok),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Outcome::Invalid;
    }
    let trimmed = digits.trim_start_matches('0');
    if trimmed.len() > 12 {
        return Outcome::Overflow;
    }
    let mag: i64 = if trimmed.is_empty() { 0 } else { trimmed.parse().unwrap() };
    let v = if neg { -mag } else { mag };
    if v < i32::MIN as i64 || v > i32::MAX as i64 {
        Outcome::Overflow
    } else {
        Outcome::Ok(v)
    }
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let (mut ok, mut inv, mut ovf, mut emp) = (0, 0, 0, 0);
    for line in s.lines() {
        let tok = line.trim_end_matches('\r');
        match classify(tok) {
            Outcome::Ok(v) => {
                ok += 1;
                println!("ok {}", v);
            }
            Outcome::Empty => {
                emp += 1;
                println!("error empty");
            }
            Outcome::Invalid => {
                inv += 1;
                println!("error invalid {}", tok);
            }
            Outcome::Overflow => {
                ovf += 1;
                println!("error overflow {}", tok);
            }
        }
    }
    println!("summary ok={} invalid={} overflow={} empty={}", ok, inv, ovf, emp);
}
