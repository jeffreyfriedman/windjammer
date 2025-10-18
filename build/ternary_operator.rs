fn main() {
    let x = 5;
    let status = if x > 0 { "positive" } else { "non-positive" };
    println!("Status: {}", status);
    let grade = if x >= 90 { "A" } else { if x >= 80 { "B" } else { "C" } };
    println!("Grade: {}", grade);
    let y = 10;
    let max = if x > y { x } else { y };
    println!("Max: {}", max);
    let is_even = if x % 2 == 0 { true } else { false };
    println!("Is even: {}", is_even)
}

