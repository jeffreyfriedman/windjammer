#[inline]
fn load(s: String) -> Result<i64, String> {
    let n = parse(&s)?;
    let p = check_positive(n)?;
    Ok(p)
}

fn main() {
    println!("{:?}", load("7".to_string()));
    println!("{:?}", load("x".to_string()));
    println!("{:?}", load("-4".to_string()));
}

#[inline]
fn parse(s: &str) -> Result<i64, String> {
    match s.parse::<i64>() {
        Ok(n) => Ok(n),
        Err(_) => { let _temp0 = format!("not a number: {}", s); Err(_temp0) },
    }
}

#[inline]
fn check_positive(n: i64) -> Result<i64, String> {
    if n < 0_i64 {
        { let _temp0 = format!("negative: {}", n); Err(_temp0) }
    } else {
        Ok(n)
    }
}

