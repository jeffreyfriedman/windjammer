fn main() {
    match std::fs::read() {
        Ok(x) => println!("ok"),
        Err(e) => println!("err"),
    }
}

