#[inline]
fn greet(name: &String) -> String {
    format!("Hello, {}!", name)
}

#[inline]
fn add(x: i64, y: i64) -> i64 {
    x + y
}

#[inline]
fn increment(counter: &mut i64) {
    counter = counter + 1;
}

#[inline]
fn sign(x: i64) -> String {
    if x >= 0 { "positive" } else { "negative" }
}

#[inline]
fn describe_number(x: i64) -> String {
    match x {
        0 => "zero",
        1 => "one",
        n if n < 0 => "negative",
        n if n > 100 => "large",
        _ => "other",
    }
}

#[inline]
fn abs(x: i64) -> i64 {
    if x < 0 {
        -x
    } else {
        x
    }
}

fn main() {
    let name = "World";
    println!("{}", greet(&name));
    let x = 5;
    let y = 10;
    println!("{} + {} = {}", x, y, add(x, y));
    let mut count = 0;
    increment(&mut count);
    println!("Counter: {}", count);
    println!("5 is {}", sign(5));
    println!("-3 is {}", sign(-3));
    for i in 0..5 {
        println!("{} is {}", i, describe_number(i));
    }
    println!("abs(-42) = {}", abs(-42))
}

