#[inline]
fn divide(a: i64, b: i64) -> Result<i64, String> {
    if b == 0_i64 {
        Err(String::from("division by zero"))
    } else {
        Ok(a / b)
    }
}

fn main() {
    match divide(10_i64, 2_i64) {
        Ok(v) => println!("ok {}", v),
        Err(e) => println!("err: {}", e),
    }
    match divide(1_i64, 0_i64) {
        Ok(v) => println!("ok {}", v),
        Err(e) => println!("err: {}", e),
    }
}

