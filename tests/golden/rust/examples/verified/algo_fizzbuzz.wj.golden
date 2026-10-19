#[inline]
fn fizzbuzz(mut i: i64) -> String {
    if i % 15_i64 == 0_i64 {
        String::from("FizzBuzz")
    } else {
        if i % 3_i64 == 0_i64 {
            String::from("Fizz")
        } else {
            if i % 5_i64 == 0_i64 {
                String::from("Buzz")
            } else {
                i.to_string()
            }
        }
    }
}

fn main() {
    let mut parts: Vec<String> = vec![];
    for i in 1_i32..=15_i32 {
        parts.push(fizzbuzz((i as i64)));
    }
    println!("{}", parts.join(" "));
}

