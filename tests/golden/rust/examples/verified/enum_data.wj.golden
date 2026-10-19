#[derive(Clone, Debug, PartialEq, Copy)]
enum Shape {
    Circle(f64),
    Rect(f64, f64),
    Square { side: f64 },
}

#[inline]
fn area(s: Shape) -> f64 {
    match s {
        Shape::Circle(r) => 3.14_f64 * r * r,
        Shape::Rect(w, h) => w * h,
        Shape::Square { side } => side * side,
    }
}

fn main() {
    println!("{}", area(Shape::Circle(5.0_f64)) * 1.0_f64);
    println!("{}", area(Shape::Rect(2.0_f64, 3.0_f64)));
    println!("{}", area(Shape::Square { side: 3.0_f64 }));
}

