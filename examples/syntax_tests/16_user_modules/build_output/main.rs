pub mod utils {
pub fn double(x: i64) -> i64 {
    x * 2
}

pub fn triple(x: i64) -> i64 {
    x * 3
}

pub fn greet(name: &String) -> String {
    let greeting = "Hello";
    format!("{}, {}!", greeting, name)
}


}


use utils::*;


fn main() {
    println!("Testing user-defined modules!");
    let x = 5;
    let doubled = utils::double(x);
    let tripled = utils::triple(x);
    println!("{} doubled is {}", x, doubled);
    println!("{} tripled is {}", x, tripled);
    let message = utils::greet("Windjammer");
    println!("{}", message);
    println!("User modules work! ✓")
}

