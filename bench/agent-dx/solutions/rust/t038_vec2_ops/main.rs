use std::io::Read;
use std::ops::{Add, Mul, Sub};

#[derive(Clone, Copy, PartialEq)]
struct V2 {
    x: i64,
    y: i64,
}
impl Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 { V2 { x: self.x + o.x, y: self.y + o.y } }
}
impl Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 { V2 { x: self.x - o.x, y: self.y - o.y } }
}
impl Mul for V2 {
    type Output = i64;
    fn mul(self, o: V2) -> i64 { self.x * o.x + self.y * o.y }
}
impl Mul<i64> for V2 {
    type Output = V2;
    fn mul(self, k: i64) -> V2 { V2 { x: self.x * k, y: self.y * k } }
}
impl V2 {
    fn cross(self, o: V2) -> i64 { self.x * o.y - self.y * o.x }
}
impl std::fmt::Display for V2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    for line in s.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.len() < 5 {
            continue;
        }
        let n = |i: usize| -> i64 { p[i].parse().unwrap() };
        let a = V2 { x: n(0), y: n(1) };
        let b = V2 { x: n(3), y: n(4) };
        match p[2] {
            "+" => println!("{}", a + b),
            "-" => println!("{}", a - b),
            "*" => println!("{}", a * b),
            "x" => println!("{}", a.cross(b)),
            "s" => println!("{}", a * b.x),
            "==" => println!("{}", a == b),
            _ => {}
        }
    }
}
