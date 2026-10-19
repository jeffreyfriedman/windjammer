#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Point {
    x: i64,
    y: i64,
}
impl Point {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(16);
        __bytes.extend_from_slice(&self.x.to_ne_bytes());
        __bytes.extend_from_slice(&self.y.to_ne_bytes());
        __bytes
    }
}


impl Point {
#[inline]
fn describe(&self) {
        println!("[point] x={}, y={}", self.x, self.y);
}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Counter {
    value: i64,
}
impl Counter {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(8);
        __bytes.extend_from_slice(&self.value.to_ne_bytes());
        __bytes
    }
}


impl Counter {
#[inline]
fn get(&self) -> i64 {
        self.value
}
#[inline]
fn increment(&mut self) {
        self.value += 1;
}
}

fn main() {
    let p = Point { x: 3_i64, y: 4_i64 };
    p.describe();
    let mut c = Counter { value: 0_i64 };
    println!("[counter] {}", c.get());
    c.increment();
    println!("[counter] {}", c.get());
    c.increment();
    println!("[counter] {}", c.get());
    println!("PASSED");
}

