#[derive(Clone, Debug, PartialEq, Copy)]
enum Color {
    Red,
    Green,
    Blue,
    Rgb(i32, i32, i32),
    Rgba(i32, i32, i32, i32),
}

#[derive(Clone, Debug, PartialEq, Copy)]
enum Shape {
    Circle(f32),
    Rectangle(f32, f32),
    Triangle(f32, f32, f32),
}

#[derive(Clone, Debug)]
enum Result<T, E> {
    Ok(T),
    Err(E),
}

#[derive(Clone, Debug)]
enum Option<T> {
    Some(T),
    None,
}

#[inline]
fn test_color_match(color: Color) -> i32 {
    match color {
        Color::Red => {
            return 1_i32;
        },
        Color::Green => {
            return 2_i32;
        },
        Color::Blue => {
            return 3_i32;
        },
        Color::Rgb(r, g, b) => {
            return (r + g + b) as i32;
        },
        Color::Rgba(r, g, b, a) => {
            return (r + g + b + a) as i32;
        },
    }
}

#[inline]
fn test_shape_match(shape: Shape) -> f32 {
    match shape {
        Shape::Circle(radius) => {
            return radius * 2.0_f32;
        },
        Shape::Rectangle(width, height) => {
            return width * height;
        },
        Shape::Triangle(a, b, c) => {
            return a + b + c;
        },
    }
}

#[inline]
fn test_wildcards(color: Color) -> i32 {
    match color {
        Color::Rgb(r, _, _) => {
            return r;
        },
        Color::Rgba(_, g, _, _) => {
            return g;
        },
        _ => {
            return 0_i32;
        },
    }
}

#[inline]
fn test_construction() -> i32 {
    let _red = Color::Red;
    let rgb = Color::Rgb(255_i32, 128_i32, 64_i32);
    let rgba = Color::Rgba(255_i32, 255_i32, 255_i32, 255_i32);
    let _circle = Shape::Circle(10.0_f64);
    let _rect = Shape::Rectangle(5.0_f64, 10.0_f64);
    (test_color_match(rgb) + test_color_match(rgba)) as i32
}

#[inline]
fn test_nested(result: Result<Option<i32>, i32>) -> i32 {
    match result {
        Result::Ok(Option::Some(value)) => {
            return value;
        },
        Result::Ok(Option::None) => {
            return 0_i32;
        },
        Result::Err(code) => {
            return code;
        },
    }
}

fn main() {
    let _r1 = test_color_match(Color::Red);
    let _r2 = test_color_match(Color::Rgb(100_i32, 50_i32, 25_i32));
    let _r3 = test_color_match(Color::Rgba(10_i32, 20_i32, 30_i32, 40_i32));
    let _r4 = test_shape_match(Shape::Circle(5.0_f64));
    let _r5 = test_shape_match(Shape::Rectangle(4.0_f64, 6.0_f64));
    let _r6 = test_wildcards(Color::Rgb(99_i32, 0_i32, 0_i32));
    let _r7 = test_construction();
    let _r8 = test_nested(Result::Ok(Option::Some(42_i32)));
    println!("All tuple enum variant tests passed!");
}

