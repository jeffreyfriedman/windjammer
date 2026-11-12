#[derive(Clone, Debug, PartialEq, Copy)]
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
}

impl Shape {
#[inline]
fn name(&self) -> String {
        match *self {
            Shape::Circle(_) => String::from("Circle"),
            Shape::Rectangle(_, _) => String::from("Rectangle"),
        }
}
#[inline]
fn area(self) -> f64 {
        match self {
            Shape::Circle(r) => 3.14159_f64 * r * r,
            Shape::Rectangle(w, h) => w * h,
        }
}
}

fn main() {
    let shapes = vec![Shape::Circle(5.0_f64), Shape::Rectangle(4.0_f64, 6.0_f64)];
    let mut total = 0.0_f64;
    for s in shapes {
        let a = s.area();
        let n = s.name();
        println!("{:<14} {:.2}", n, a);
        total += a;
    }
    let label: String = String::from("total");
    println!("{:<14} {:.2}", label, total);
}

