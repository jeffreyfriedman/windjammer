// Rust twin of bench/interp/sieve.wj: primes below 1,000,000.
fn main() {
    let n = 1_000_000;
    let mut flags = vec![true; n];
    let mut count = 0;
    for i in 2..n {
        if flags[i] {
            count += 1;
            let mut j = i * i;
            while j < n {
                flags[j] = false;
                j += i;
            }
        }
    }
    println!("{count}");
}
