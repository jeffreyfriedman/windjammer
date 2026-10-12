fn main() {
    match "42".parse::<i64>() {
        Ok(n) => println!("{}", n),
        Err(_) => println!("parse failed"),
    }
    match "abc".parse::<i64>() {
        Ok(v) => println!("{}", v),
        Err(_) => println!("parse failed"),
    }
    let f = "3.5".parse::<f64>().unwrap_or(0.0_f64);
    println!("{}", f);
    let neg = "-17".parse::<i64>().unwrap_or(0_i64);
    println!("{}", neg);
}

