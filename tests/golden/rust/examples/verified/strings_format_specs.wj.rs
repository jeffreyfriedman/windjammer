fn main() {
    let h1: String = String::from("name");
    let h2: String = String::from("qty");
    let h3: String = String::from("price");
    println!("|{:<10}|{:>5}|{:>8}|", h1, h2, h3);
    let r1: String = String::from("apple");
    println!("|{:<10}|{:>5}|{:>8.2}|", r1, 3, 1.25_f64);
    let r2: String = String::from("watermelon");
    println!("|{:<10}|{:>5}|{:>8.2}|", r2, 10, 12.5_f64);
    println!("hex {:x}, bin {:b}, oct {:o}, sci {:e}", 255, 10, 15, 1500.0_f64);
}

