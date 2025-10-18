#[inline]
fn double(x: i64) -> i64 {
    x * 2
}

#[inline]
fn triple(x: i64) -> i64 {
    x * 3
}

#[inline]
fn greet(name: &String) -> String {
    let greeting = "Hello";
    format!("{}, {}!", greeting, name)
}

