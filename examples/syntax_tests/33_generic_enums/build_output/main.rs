enum Option<T> {
    Some(T),
    None,
}

enum Result<T, E> {
    Ok(T),
    Err(E),
}

enum Container<T> {
    Value(T),
    Empty,
    Multiple(Vec<T>),
}

fn safe_divide(a: i64, b: i64) -> Result<i64, String> {
    if b == 0 {
        Result::Err("division by zero")
    } else {
        Result::Ok(a / b)
    }
}

fn find_first(vec: &Vec<i64>) -> Option<i64> {
    if vec.is_empty() {
        Option::None
    } else {
        Option::Some(vec[0])
    }
}

fn main() {
    let some_value = Option::Some(42);
    let no_value: Option<i64> = Option::None;
    match some_value {
        Some(x) => println!("Found value: {}", x),
        None => println!("No value"),
    }
    let success = Result::Ok(100);
    let failure: Result<i64, String> = Result::Err("error occurred");
    match safe_divide(10, 2) {
        Ok(value) => println!("Result: {}", value),
        Err(msg) => println!("Error: {}", msg),
    }
    match safe_divide(10, 0) {
        Ok(value) => println!("Result: {}", value),
        Err(msg) => println!("Error: {}", msg),
    }
    let single = Container::Value(123);
    let empty: Container<i64> = Container::Empty;
    match single {
        Value(x) => println!("Container has: {}", x),
        Empty => println!("Container is empty"),
        Multiple(vec) => println!("Container has {} items", vec.len()),
    }
    println!("All generic enum examples working!")
}

