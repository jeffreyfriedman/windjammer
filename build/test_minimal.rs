struct Point {
    x: i64,
    y: i64,
}

impl Point {
#[inline]
fn new(&self, x: i64, y: i64) -> Point {
        Point { x, y }
}
}

enum Shape {
    Circle(f64),
    Square(f64),
}

#[inline]
fn area(shape: &Shape) -> f64 {
    match shape {
        Shape.Circle(radius) => 3.14159 * radius * radius,
        Shape.Square(side) => side * side,
    }
}

#[inline]
fn fibonacci(n: i64) -> i64 {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}

fn main() {
    let p = Point::new(3, 4);
    println!("Point({}, {})", p.x, p.y);
    let circle = Shape::Circle(5.0);
    println!("Circle area: {}", area(&circle));
    println!("fibonacci(10) = {}", fibonacci(10))
}

