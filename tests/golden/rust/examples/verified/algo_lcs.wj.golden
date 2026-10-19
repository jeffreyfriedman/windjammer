fn lcs(a: &str, b: &str) -> i64 {
    let x: Vec<char> = a.chars().collect::<Vec<char>>();
    let y: Vec<char> = b.chars().collect::<Vec<char>>();
    let mut dp: Vec<Vec<i64>> = vec![];
    for _ in 0_usize..=x.len() {
        dp.push(vec![0_i64; 8_i64]);
    }
    for i in 0_usize..x.len() {
        for j in 0_usize..y.len() {
            if x[i] == y[j] {
                dp[i + 1][j + 1] = dp[i][j] + 1_i64;
            } else {
                dp[i + 1][j + 1] = dp[i][j + 1].max((dp[(i + 1) as usize][j]) as f32);
            }
        }
    }
    dp[x.len()][y.len()]
}

fn main() {
    println!("lcs = {}", lcs("AGGTAB", "GXTXAYB"));
}

