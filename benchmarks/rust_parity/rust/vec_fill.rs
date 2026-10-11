fn fill(n: usize) -> Vec<u64> {
    (0..n).map(|i| (i as u64) * 2654435761 % 1000).collect()
}

fn main() {
    let mut total: u64 = 0;
    for round in 0..40u64 {
        let v = fill(2_000_000);
        total += v.iter().sum::<u64>() + round;
    }
    println!("{}", total);
}
