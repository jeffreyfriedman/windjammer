#[derive(Clone, Debug, PartialEq, Copy)]
enum Shape {
    Circle { radius: f32 },
    Rectangle { width: f32, height: f32 },
    Triangle { base: f32, height: f32 },
}

#[inline]
fn calculate_area(shape: Shape) -> f32 {
    match shape {
        Shape::Circle { radius: r } => {
            return 3.14159_f32 * r * r;
        },
        Shape::Rectangle { width: w, height: h } => {
            return w * h;
        },
        Shape::Triangle { base: b, height: h } => {
            return 0.5_f32 * b * h;
        },
    }
}

#[inline]
fn test_circle() -> f32 {
    let circle = Shape::Circle { radius: 5.0_f32 };
    calculate_area(circle)
}

#[inline]
fn test_rectangle() -> f32 {
    let rect = Shape::Rectangle { width: 10.0_f32, height: 20.0_f32 };
    calculate_area(rect)
}

#[inline]
fn test_triangle() -> f32 {
    let tri = Shape::Triangle { base: 6.0_f32, height: 8.0_f32 };
    calculate_area(tri)
}

#[inline]
fn has_large_dimension(shape: Shape) -> bool {
    match shape {
        Shape::Circle { radius: r } => r > (10.0_f64 as f32),
        Shape::Rectangle { width: w, height: _ } => w > (10.0_f64 as f32),
        Shape::Triangle { base: _, height: h } => h > (10.0_f64 as f32),
    }
}

#[inline]
fn get_first_dimension(shape: Shape) -> f32 {
    match shape {
        Shape::Circle { radius: r } => r,
        Shape::Rectangle { width: w, height: _ } => w,
        Shape::Triangle { base: b, height: _ } => b,
    }
}

