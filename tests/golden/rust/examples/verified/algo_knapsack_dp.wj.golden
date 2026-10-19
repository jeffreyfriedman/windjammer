#[inline]
fn knapsack(weights: &Vec<i64>, values: &Vec<i64>, cap: i64) -> i64 {
    let mut dp = vec![0; 8];
    for i in 0_usize..weights.len() {
        let mut c = cap;
        while c >= weights[i] {
            let cand = dp[(c - weights[i]) as usize] + values[i];
            if cand > (dp[c as usize]) {
                dp[c as usize] = cand;
            }
            c -= 1;
        }
    }
    dp[cap as usize]
}

fn main() {
    println!("best value: {}", knapsack(&[1, 3, 4, 5], &[1, 4, 5, 7], 7_i64));
}

