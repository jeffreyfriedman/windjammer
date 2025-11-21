fn double(x: i64) -> i64 {
    x * 2
}

fn triple(x: i64) -> i64 {
    x * 3
}

fn greet(name: &String) -> String {
    let greeting = "Hello";
    format!("{}, {}!", greeting, name)
}

