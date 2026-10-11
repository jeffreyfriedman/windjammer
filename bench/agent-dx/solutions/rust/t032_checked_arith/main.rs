use std::io::Read;

fn pow(a: i32, b: i32) -> Option<i32> {
    match a {
        0 => Some(if b == 0 { 1 } else { 0 }),
        1 => Some(1),
        -1 => Some(if b % 2 == 0 { 1 } else { -1 }),
        _ => {
            let mut r: i32 = 1;
            for _ in 0..b {
                r = r.checked_mul(a)?;
            }
            Some(r)
        }
    }
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    for line in s.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.len() < 3 {
            continue;
        }
        let a: i32 = p[1].parse().unwrap();
        let b: i32 = p[2].parse().unwrap();
        let r: Result<Option<i32>, &str> = match p[0] {
            "add" => Ok(a.checked_add(b)),
            "sub" => Ok(a.checked_sub(b)),
            "mul" => Ok(a.checked_mul(b)),
            "div" => if b == 0 { Err("divzero") } else { Ok(a.checked_div(b)) },
            "rem" => if b == 0 { Err("divzero") } else { Ok(Some(a.wrapping_rem(b))) },
            "pow" => if b < 0 { Err("invalid") } else { Ok(pow(a, b)) },
            _ => Err("invalid"),
        };
        match r {
            Ok(Some(v)) => println!("{}", v),
            Ok(None) => println!("overflow"),
            Err(e) => println!("{}", e),
        }
    }
}
