// Rust twin of bench/interp/closures.wj: iterator chains over a Vec, repeated.
fn main() {
    let v: Vec<i64> = (0..1000).collect();
    let mut total: i64 = 0;
    for r in 0..300_i64 {
        let s: i64 = v.iter().map(|&x| x * r).filter(|x| x % 3 == 0).map(|x| x / 3).sum();
        total += s % 100_000;
    }
    println!("{total}");
}
