#[inline]
fn find_pair(a: &Vec<i64>, b: &Vec<i64>) -> String {
    for x in a.iter().copied() {
        for y in b.iter().copied() {
            if x + y == 10_i64 {
                return format!("{} + {}", x, y);
            }
        }
    }
    String::from("none")
}

fn main() {
    println!("first pair summing to 10: {}", find_pair(&[1, 3, 5], &[2, 7, 9]));
}

