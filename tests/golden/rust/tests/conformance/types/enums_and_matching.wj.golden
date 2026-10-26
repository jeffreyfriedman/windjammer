#[derive(Clone, Debug, PartialEq, Copy)]
enum Color {
    Red,
    Green,
    Blue,
}

#[derive(Clone, Debug, PartialEq, Copy)]
enum Shape {
    Circle(i64),
    Square(i64),
    Point,
}

#[derive(Clone, Debug)]
enum Maybe<T> {
    Some(T),
    None,
}

#[derive(Clone, Debug)]
enum Result<T, E> {
    Ok(T),
    Err(E),
}

impl Shape {
#[inline]
fn precise_area(self) -> i64 {
        match self {
            Shape::Circle(r) => 314_i64 * r * r / 100_i64,
            Shape::Square(side) => side * side,
            Shape::Point => 0_i64,
        }
}
}

#[inline]
fn color_name(c: Color) -> String {
    match c {
        Color::Red => String::from("red"),
        Color::Green => String::from("green"),
        Color::Blue => String::from("blue"),
    }
}

#[inline]
fn test_unit_enum() {
    println!("[enum_unit] Red -> {}", color_name(Color::Red));
    println!("[enum_unit] Green -> {}", color_name(Color::Green));
    println!("[enum_unit] Blue -> {}", color_name(Color::Blue));
}

#[inline]
fn shape_area(s: Shape) -> i64 {
    match s {
        Shape::Circle(r) => 3_i64 * r * r,
        Shape::Square(side) => side * side,
        Shape::Point => 0_i64,
    }
}

#[inline]
fn test_data_enum() {
    println!("[enum_data] Circle area: {}", shape_area(Shape::Circle(5)));
    println!("[enum_data] Square area: {}", shape_area(Shape::Square(4)));
    println!("[enum_data] Point area: {}", shape_area(Shape::Point));
}

#[inline]
fn unwrap_or(m: Maybe<i64>, default: i64) -> i64 {
    match m {
        Maybe::Some(v) => v,
        Maybe::None => default,
    }
}

#[inline]
fn test_option_enum() {
    println!("[enum_option] found: {}", unwrap_or(Maybe::Some(42), 0_i64));
    println!("[enum_option] not found: {}", unwrap_or(Maybe::None, 0_i64));
}

#[inline]
fn categorize(x: i64) -> String {
    match x {
        1 | 2 | 3 => String::from("small"),
        4 | 5 | 6 => String::from("medium"),
        _ => String::from("other"),
    }
}

#[inline]
fn test_or_patterns() {
    println!("[enum_or] 1 -> {}", categorize(1_i64));
    println!("[enum_or] 3 -> {}", categorize(3_i64));
    println!("[enum_or] 5 -> {}", categorize(5_i64));
    println!("[enum_or] 99 -> {}", categorize(99_i64));
}

#[inline]
fn extract_nested(r: Result<Maybe<i64>, String>) -> i64 {
    match r {
        Result::Ok(Maybe::Some(v)) => v,
        Result::Ok(Maybe::None) => 0_i64,
        Result::Err(_) => -1_i64,
    }
}

#[inline]
fn test_nested_patterns() {
    println!("[enum_nested] Ok(Some(42)) -> {}", extract_nested(Result::Ok(Maybe::Some(42))));
    println!("[enum_nested] Ok(None) -> {}", extract_nested(Result::Ok(Maybe::None)));
    println!("[enum_nested] Err -> {}", extract_nested(Result::Err(String::from("error"))));
}

#[inline]
fn test_enum_methods() {
    let c = Shape::Circle(10_i32);
    let s = Shape::Square(5_i32);
    println!("[enum_method] Circle: area={}", c.precise_area());
    println!("[enum_method] Square: area={}", s.precise_area());
}

#[inline]
fn test_match_expression() {
    for x in [5, 0, -3] {
        let label: String = match x {
            n if n > 0_i32 => "positive",
            0 => "zero",
            _ => "negative",
        };
        println!("[enum_match_expr] {}", label);
    }
}

fn main() {
    test_unit_enum();
    test_data_enum();
    test_option_enum();
    test_or_patterns();
    test_nested_patterns();
    test_enum_methods();
    test_match_expression();
    println!("[enum_all] PASSED");
}

