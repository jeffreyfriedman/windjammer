use std::io::Read;

const TABLE: [(u32, &str); 13] = [
    (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"),
    (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
];

fn to_roman(mut n: u32) -> String {
    let mut r = String::new();
    for &(v, sym) in TABLE.iter() {
        while n >= v {
            r.push_str(sym);
            n -= v;
        }
    }
    r
}

fn val(c: char) -> i32 {
    match c {
        'I' => 1, 'V' => 5, 'X' => 10, 'L' => 50, 'C' => 100, 'D' => 500, _ => 1000,
    }
}

fn from_roman(s: &str) -> i32 {
    let v: Vec<i32> = s.chars().map(val).collect();
    let mut total = 0;
    for i in 0..v.len() {
        if i + 1 < v.len() && v[i] < v[i + 1] {
            total -= v[i];
        } else {
            total += v[i];
        }
    }
    total
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    for line in s.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        match l.parse::<u32>() {
            Ok(n) => println!("{}", to_roman(n)),
            Err(_) => println!("{}", from_roman(l)),
        }
    }
}
