#[inline]
fn count_and_consume(data: &Vec<i64>) -> i64 {
    let len = data.len();
    len
}

fn main() {
    let mut data = Vec::new();
    let mut i = 0;
    while i < 1000000 {
        data.push(i);
        i += 1;
    }
    println!("Created Vec with {} elements", data.len());
    let count = count_and_consume(&data);
    println!("Count: {}", count);
    println!("Function returned instantly! (drop happening in background)")
}

