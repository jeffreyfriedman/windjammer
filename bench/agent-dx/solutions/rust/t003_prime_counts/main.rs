use std::io::Read;

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();
    let t: usize = it.next().unwrap().parse().unwrap();
    const N: usize = 2_000_000;
    let mut comp = vec![false; N + 1];
    let mut count = vec![0u32; N + 1];
    let mut largest = vec![0u32; N + 1];
    for i in 2..=N {
        if !comp[i] {
            let mut j = i * i;
            while j <= N {
                comp[j] = true;
                j += i;
            }
        }
        count[i] = count[i - 1] + if comp[i] { 0 } else { 1 };
        largest[i] = if comp[i] { largest[i - 1] } else { i as u32 };
    }
    for _ in 0..t {
        let n: usize = it.next().unwrap().parse().unwrap();
        if n < 2 {
            println!("0 none");
        } else {
            println!("{} {}", count[n], largest[n]);
        }
    }
}
