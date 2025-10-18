#[derive(Debug, Clone, PartialEq)]
struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
#[inline]
fn new(&self, x: f64, y: f64) -> Point {
        Point { x, y }
}
#[inline]
fn distance(self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y(dx * dx + dy * dy).sqrt();
}
}

#[inline]
fn hello(name: &String) -> String {
    format!("Hello, {}!", name)
}

#[inline]
fn add(a: i64, b: i64) -> i64 {
    a + b
}

#[inline]
fn multiply(a: i64, b: i64) -> i64 {
    a * b
}

#[test]
fn test_hello() {
    let result = hello(&"World");
    assert_eq!(result, "Hello, World!")
}

#[test]
fn test_add() {
    assert_eq!(add(2, 3), 5);
    assert_eq!(add(-1, 1), 0)
}

#[test]
fn test_multiply() {
    assert_eq!(multiply(3, 4), 12);
    assert_eq!(multiply(0, 100), 0)
}

#[test]
fn test_point_distance() {
    let p1 = Point::new(0.0, 0.0);
    let p2 = Point::new(3.0, 4.0);
    let dist = p1.distance(p2);
    assert_eq!(dist, 5.0)
}

