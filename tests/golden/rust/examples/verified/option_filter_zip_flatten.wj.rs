fn main() {
    println!("{:?}", Some(3).filter(|x| *x > 5_i64));
    println!("{:?}", Some(8).filter(|x| *x > 5_i64));
    let z = Some(1_i32).zip(Some(String::from("a")));
    println!("{:?}", z);
    let o: Option<i64> = None;
    println!("{:?}", o.or(Some(3)));
    let nested = Some(Some(9_i32));
    println!("{}", nested.flatten().unwrap_or(0));
}

