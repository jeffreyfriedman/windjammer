fn main() {
    let grid = vec![vec![1_i32, 2_i32, 3_i32], vec![4_i32, 5_i32, 6_i32]];
    let mut total = 0_i32;
    for row in grid.iter().copied() {
        let strs: Vec<String> = row.iter().map(|x| x.to_string()).collect::<Vec<String>>();
        println!("{}", strs.join(" "));
        for x in row.iter().copied() {
            total += x as i32;
        }
    }
    let diag = grid[0][0] + grid[1][1];
    println!("diag={} total={}", diag, total);
}

