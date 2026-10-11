fn sieve(n: usize) -> Vec<bool> {
    let mut flags: Vec<bool> = Vec::new();
    for _i in 0_usize..n + 1_usize {
        flags.push(true);
    }
    flags[0] = false;
    flags[1] = false;
    let mut i: usize = 2_usize;
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
    let n = 20000000_usize;
    let flags: Vec<bool> = sieve(n as usize);
    let mut count = 0_i32;
    let mut sum: u64 = 0_u64;
    for i in 0_usize..n + 1_usize {
        if flags[i] {
            count += 1;
            sum += i as u64;
        }
    }
    println!("{} {}", count, sum);
}

