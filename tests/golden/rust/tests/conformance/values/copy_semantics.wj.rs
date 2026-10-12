#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Point {
    x: i64,
    y: i64,
}










#[inline]
fn test_copy_int() {
    let a = 42_i32;
    let mut b = a;
    println!("[copy_int] a={}, b={}", a, b);
    let mut b = a;
    b = 99_i32;
    println!("[copy_int] after b=99: a={}, b={}", a, b);
}

#[inline]
fn test_copy_float() {
    let a = 3.14_f64;
    let mut b = a;
    println!("[copy_float] a={}, b={}", a, b);
    let mut b = a;
    b = 2.72_f64;
    println!("[copy_float] after b=2.72: a={}, b={}", a, b);
}

#[inline]
fn test_copy_bool() {
    let a = true;
    let mut b = a;
    println!("[copy_bool] a={}, b={}", a, b);
    let mut b = a;
    b = false;
    println!("[copy_bool] after b=false: a={}, b={}", a, b);
}

#[inline]
fn double(x: i64) -> i64 {
    x * 2_i64
}

#[inline]
fn test_copy_param() {
    let original = 10_i64;
    let doubled = double(original);
    println!("[copy_param] original={}, doubled={}", original, doubled);
    println!("[copy_param] after call: original={}", original);
}

#[inline]
fn test_copy_nested() {
    let p1 = Point { x: 1_i64, y: 2_i64 };
    let mut p2 = Point { x: p1.x, y: p1.y };
    println!("[copy_nested] p1.x={}, p1.y={}", p1.x, p1.y);
    println!("[copy_nested] p2.x={}, p2.y={}", p2.x, p2.y);
    p2.x = 99_i64;
    println!("[copy_nested] after p2.x=99: p1.x={}, p2.x={}", p1.x, p2.x);
}

fn main() {
    test_copy_int();
    test_copy_float();
    test_copy_bool();
    test_copy_param();
    test_copy_nested();
    println!("[copy_all] PASSED");
}

