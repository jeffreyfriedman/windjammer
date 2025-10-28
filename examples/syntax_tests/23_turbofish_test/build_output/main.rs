fn identity<T>(x: &T) -> T {
    x
}

fn main() {
    let x = identity::<i64>(42);
    println!("Identity int: {}", x);
    let s = identity::<String>("Hello".to_string());
    println!("Identity string: {}", s);
    let text = "42";
    let parsed = text.parse::<i64>();
    println!("Parsed result: {:?}", parsed)
}

