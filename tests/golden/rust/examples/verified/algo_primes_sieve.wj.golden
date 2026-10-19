#[inline]
fn sieve(limit: i64) -> Vec<i64> {
    let mut is_prime = vec![true; 60];
    let mut primes: Vec<i64> = vec![];
    for i in 2_i64..limit {
        if is_prime[i as usize] {
            let _ = primes.push(i);
            let mut j = i * i;
            while j < limit {
                is_prime[j as usize] = false;
                j += i;
            }
        }
    }
    primes
}

fn main() {
    let ps: Vec<i64> = sieve(60_i64);
    let strs: Vec<String> = ps.iter().map(|p| p.to_string()).collect::<Vec<String>>();
    println!("{}", strs.join(" "));
    println!("count below 60: {}", ps.len());
}

