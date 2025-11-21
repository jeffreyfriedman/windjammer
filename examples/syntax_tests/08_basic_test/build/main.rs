#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
struct Point {
    x: i64,
    y: i64,
}

impl Point {
fn new(x: i64, y: i64) -> Point {
        Point { x: x, y: y }
}
fn distance_from_origin(&self) -> i64 {
        self.x * self.x + self.y * self.y
}
fn translate(&mut self, dx: i64, dy: i64) {
        self.x = self.x + dx;
        self.y = self.y + dy;
}
fn into_tuple(self) -> (i64, i64) {
        (self.x, self.y)
}
}

#[derive(Debug, Clone, PartialEq)]
struct Rectangle {
    width: i64,
    height: i64,
}

impl Rectangle {
fn new(x: i64, y: i64) -> Point {
        Point { x: x, y: y }
}
fn area(&self) -> i64 {
        self.width * self.height
}
fn is_square(&self) -> bool {
        self.width == self.height
}
}

fn main() {
    let p1 = Point::new(3, 4);
    println!("Point 1: ({}, {})", p1.x, p1.y);
    println!("Distance from origin: {}", p1::distance_from_origin());
    let mut p2 = Point::new(0, 0);
    p2::translate(5, 5);
    println!("Point 2 after translation: ({}, {})", p2.x, p2.y);
    let p3 = Point::new(1, 2);
    let tuple = p3::into_tuple();
    println!("As tuple: ({}, {})");
    let rect1 = Rectangle::new(10, 20);
    let rect2 = Rectangle::new(15, 15);
    println!("Rectangle 1 area: {}", rect1::area());
    println!("Rectangle 1 is square: {}", rect1::is_square());
    println!("Rectangle 2 is square: {}", rect2::is_square())
}

