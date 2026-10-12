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
fn new(x: i64, y: i64) -> Point {
        Point { x, y }
}
#[inline]
fn origin() -> Point {
        Point { x: 0_i64, y: 0_i64 }
}
#[inline]
fn distance_squared(&self) -> i64 {
        self.x * self.x + self.y * self.y
}
#[inline]
fn is_origin(&self) -> bool {
        self.x == 0_i64 && self.y == 0_i64
}
#[inline]
fn translate(&mut self, dx: i64, dy: i64) {
        self.x += dx;
        self.y += dy;
}
#[inline]
fn scale(&mut self, factor: i64) {
        self.x *= factor;
        self.y *= factor;
}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Rect {
    width: i64,
    height: i64,
}
impl Rect {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(16);
        __bytes.extend_from_slice(&self.width.to_ne_bytes());
        __bytes.extend_from_slice(&self.height.to_ne_bytes());
        __bytes
    }
}


impl Rect {
#[inline]
fn new(width: i64, height: i64) -> Rect {
        Rect { width, height }
}
#[inline]
fn area(&self) -> i64 {
        self.width * self.height
}
#[inline]
fn is_square(&self) -> bool {
        self.width == self.height
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
fn new() -> Counter {
        Counter { value: 0_i64 }
}
#[inline]
fn increment(&mut self) {
        self.value += 1;
}
#[inline]
fn get(&self) -> i64 {
        self.value
}
}

#[inline]
fn test_basic_struct() {
    let p = Point { x: 10_i64, y: 20_i64 };
    println!("[struct_basic] x={}, y={}", p.x, p.y);
}

#[inline]
fn test_struct_shorthand() {
    let x = 5_i32;
    let y = 10_i32;
    let p = Point { x: (x as i64), y: (y as i64) };
    println!("[struct_shorthand] x={}, y={}", p.x, p.y);
}

#[inline]
fn test_constructor() {
    let p = Point::new(3_i64, 4_i64);
    println!("[struct_constructor] x={}, y={}", p.x, p.y);
}

#[inline]
fn test_method_read() {
    let p = Point::new(3_i64, 4_i64);
    println!("[struct_method_read] distance_sq={}", p.distance_squared());
    println!("[struct_method_read] is_origin={}", p.is_origin());
}

#[inline]
fn test_method_mut() {
    let mut p = Point::origin();
    println!("[struct_method_mut] before: x={}, y={}", p.x, p.y);
    p.translate(3_i64, 4_i64);
    println!("[struct_method_mut] after translate(3,4): x={}, y={}", p.x, p.y);
    p.scale(2_i64);
    println!("[struct_method_mut] after scale(2): x={}, y={}", p.x, p.y);
}

#[inline]
fn test_struct_composition() {
    let rect = Rect::new(10_i64, 20_i64);
    println!("[struct_compose] rect area={}", rect.area());
    println!("[struct_compose] rect is_square={}", rect.is_square());
    let square = Rect::new(5_i64, 5_i64);
    println!("[struct_compose] square is_square={}", square.is_square());
}

#[inline]
fn test_counter() {
    let mut c = Counter::new();
    println!("[struct_default] counter={}", c.get());
    c.increment();
    c.increment();
    c.increment();
    println!("[struct_default] after 3 increments: counter={}", c.get());
}

fn main() {
    test_basic_struct();
    test_struct_shorthand();
    test_constructor();
    test_method_read();
    test_method_mut();
    test_struct_composition();
    test_counter();
    println!("[struct_all] PASSED");
}

