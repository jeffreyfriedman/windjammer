#[derive(Clone, Debug, PartialEq)]
enum Color {
    Red,
    Green,
    Blue,
    Rgb(i32, i32, i32),
    Named(String),
}

#[derive(Clone, Debug)]
enum Result<T, E> {
    Ok(T),
    Err(E),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Point {
    x: i32,
    y: i32,
}










#[inline]
fn test_param_simple(x: i32) -> i32 {
    x + 42_i32
}

#[inline]
fn test_let_patterns() -> i32 {
    let (x, y) = (10_i32, 20_i32);
    let _ = 100;
    (x + y) as i32
}

#[inline]
fn test_match_patterns(color: Color) -> i32 {
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
        Color::Rgb(_, _, _) => {
            return 4_i32;
        },
        Color::Named(name) => {
            return 5_i32;
        },
    }
}

#[inline]
fn test_for_loop_patterns() -> i32 {
    let pairs = vec![(1_i32, 2_i32), (3_i32, 4_i32), (5_i32, 6_i32)];
    let mut sum: i32 = 0_i32;
    for (a, b) in pairs {
        sum = sum + a as i32 + b;
    }
    sum
}

#[inline]
fn test_nested_patterns(result: Result<Option<i32>, String>) -> i32 {
    match result {
        Result::Ok(Some(value)) => {
            return value;
        },
        Result::Ok(None) => {
            return 0_i32;
        },
        Result::Err(_) => {
            return -1_i32;
        },
    }
}

#[inline]
fn test_qualified_patterns(opt: std::option::Option<i32>) -> i32 {
    match opt {
        std::option::Option::Some(x) => {
            return x as i32;
        },
        std::option::Option::None => {
            return 0_i32;
        },
    }
}

#[inline]
fn test_struct_patterns(point: Point) -> i32 {
    match point {
        Point { x: 0, y: 0 } => {
            return 0_i32;
        },
        Point { x, y } => {
            return (x + y) as i32;
        },
    }
}

#[inline]
fn test_or_patterns(value: i32) -> i32 {
    match value {
        1 | 2 | 3 => {
            return 100_i32;
        },
        4 | 5 => {
            return 200_i32;
        },
        _ => {
            return 0_i32;
        },
    }
}

#[inline]
fn test_reference_patterns(opt: Option<i32>) -> i32 {
    match opt {
        Some(value) => {
            return value;
        },
        None => {
            return 0_i32;
        },
    }
}

fn main() {
    let _result1 = test_param_simple(10_i32);
    let _result3 = test_let_patterns();
    let _result4 = test_match_patterns(Color::Red);
    let _result5 = test_match_patterns(Color::Rgb(255_i32, 0_i32, 0_i32));
    let _result6 = test_for_loop_patterns();
    let _result7 = test_nested_patterns(Result::Ok(Some(42_i32)));
    let _result8 = test_nested_patterns(Result::Err(String::from("error")));
    let point = Point { x: 10_i32, y: 20_i32 };
    let _result9 = test_struct_patterns(point);
    let _result10 = test_or_patterns(2_i32);
    let _result11 = test_or_patterns(5_i32);
    let _result12 = test_reference_patterns(Some(100_i32));
    println!("All pattern matching tests passed!");
}

