use std::io::Read;

trait Shape {
    fn name(&self) -> &str;
    fn corners(&self) -> u32;
    fn double_area(&self) -> u64;
}

struct Square(u64);
struct Rect(u64, u64);
struct RTri(u64, u64);
struct Trap(u64, u64, u64);

impl Shape for Square {
    fn name(&self) -> &str { "square" }
    fn corners(&self) -> u32 { 4 }
    fn double_area(&self) -> u64 { 2 * self.0 * self.0 }
}
impl Shape for Rect {
    fn name(&self) -> &str { "rect" }
    fn corners(&self) -> u32 { 4 }
    fn double_area(&self) -> u64 { 2 * self.0 * self.1 }
}
impl Shape for RTri {
    fn name(&self) -> &str { "rtri" }
    fn corners(&self) -> u32 { 3 }
    fn double_area(&self) -> u64 { self.0 * self.1 }
}
impl Shape for Trap {
    fn name(&self) -> &str { "trap" }
    fn corners(&self) -> u32 { 4 }
    fn double_area(&self) -> u64 { (self.0 + self.1) * self.2 }
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut shapes: Vec<Box<dyn Shape>> = Vec::new();
    for line in s.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.is_empty() {
            continue;
        }
        let n: Vec<u64> = p[1..].iter().map(|x| x.parse().unwrap()).collect();
        match p[0] {
            "square" => shapes.push(Box::new(Square(n[0]))),
            "rect" => shapes.push(Box::new(Rect(n[0], n[1]))),
            "rtri" => shapes.push(Box::new(RTri(n[0], n[1]))),
            "trap" => shapes.push(Box::new(Trap(n[0], n[1], n[2]))),
            _ => {}
        }
    }
    let mut total = 0u64;
    let mut best: Option<usize> = None;
    for (i, sh) in shapes.iter().enumerate() {
        println!("{} corners={} area2={}", sh.name(), sh.corners(), sh.double_area());
        total += sh.double_area();
        if best.map_or(true, |b| sh.double_area() > shapes[b].double_area()) {
            best = Some(i);
        }
    }
    println!("total area2={}", total);
    match best {
        Some(b) => println!("largest={} area2={}", shapes[b].name(), shapes[b].double_area()),
        None => println!("largest=none"),
    }
}
