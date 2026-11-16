// Rust twin of bench/interp/loop5m.wj.
fn main() {
    let mut total: i64 = 0;
    for i in 0..5_000_000_i64 {
        let x = i * 3 + 1;
        if x % 7 == 0 {
            total += x;
        } else {
            total -= 1;
        }
    }
    println!("{total}");
}
