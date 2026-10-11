fn sieve(n: usize) -> Vec<bool> {
    let mut flags = vec![true; n + 1];
    flags[0] = false;
    flags[1] = false;
    let mut i = 2;
    while i * i <= n {
        if flags[i] {
            let mut j = i * i;
            while j <= n {
                flags[j] = false;
                j += i;
            }
        }
        i += 1;
    }
    flags
}

fn main() {
    let n = 20_000_000;
    let flags = sieve(n);
    let (count, sum) = flags
        .iter()
        .enumerate()
        .filter(|(_, f)| **f)
        .fold((0, 0u64), |(c, s), (i, _)| (c + 1, s + i as u64));
    println!("{} {}", count, sum);
}
