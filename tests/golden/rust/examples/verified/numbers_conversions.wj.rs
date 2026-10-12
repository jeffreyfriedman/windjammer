fn main() {
    let f = 3.99_f64;
    println!("{}", f as i64);
    let i = 3_i32;
    println!("{:.1}", i as f64);
    println!("{}", 255 as u8);
    println!("{}", 'A' as i64);
    let c = 65 as u8 as char;
    println!("{}", c);
}

