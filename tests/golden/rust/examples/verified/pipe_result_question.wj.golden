#[inline]
fn quarter(n: i64) -> Result<i64, String> {
    half(half(n)?)
}

fn main() {
    println!("{:?}", quarter(8_i64));
    println!("{:?}", quarter(6_i64));
}

#[inline]
fn half(n: i64) -> Result<i64, String> {
    if n % 2_i64 != 0_i64 {
        { let _temp0 = format!("odd {}", n); Err(_temp0) }
    } else {
        Ok(n / 2_i64)
    }
}

