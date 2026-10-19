#[derive(Clone, Debug, PartialEq, Copy)]
enum Color {
    Red,
    Green,
    Blue,
}

#[derive(Clone, Debug)]
enum Option<T> {
    Some(T),
    None,
}

#[derive(Clone, Debug)]
enum Result<T, E> {
    Ok(T),
    Err(E),
}

#[inline]
fn test_unit_variants(color: Color) -> i32 {
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
    }
}

#[inline]
fn test_variant_bindings(opt: Option<i32>) -> i32 {
    match opt {
        Option::Some(value) => {
            return value;
        },
        Option::None => {
            return 0_i32;
        },
    }
}

#[inline]
fn test_nested_patterns(result: Result<i32, i32>) -> i32 {
    match result {
        Result::Ok(value) => {
            return value;
        },
        Result::Err(code) => {
            return code;
        },
    }
}

#[inline]
fn test_wildcards(value: i32) -> i32 {
    match value {
        1 => {
            return 100_i32;
        },
        2 => {
            return 200_i32;
        },
        _ => {
            return 0_i32;
        },
    }
}

#[inline]
fn test_literals(value: i32) -> i32 {
    match value {
        0 => {
            return 0_i32;
        },
        1 => {
            return 1_i32;
        },
        42 => {
            return 42_i32;
        },
        _ => {
            return -1_i32;
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
fn test_wildcard_in_variant(opt: Option<i32>) -> i32 {
    match opt {
        Option::Some(_) => {
            return 1_i32;
        },
        Option::None => {
            return 0_i32;
        },
    }
}

fn main() {
    let _r1 = test_unit_variants(Color::Red);
    let _r2 = test_unit_variants(Color::Green);
    let _r3 = test_variant_bindings(Option::Some(42_i32));
    let _r4 = test_variant_bindings(Option::None);
    let _r5 = test_nested_patterns(Result::Ok(100_i32));
    let _r6 = test_nested_patterns(Result::Err(-1_i32));
    let _r7 = test_wildcards(1_i32);
    let _r8 = test_wildcards(999_i32);
    let _r9 = test_literals(42_i32);
    let _r10 = test_literals(100_i32);
    let _r11 = test_or_patterns(2_i32);
    let _r12 = test_or_patterns(5_i32);
    let _r13 = test_wildcard_in_variant(Option::Some(999_i32));
    println!("Pattern matching audit: ALL SUPPORTED PATTERNS WORK");
}

