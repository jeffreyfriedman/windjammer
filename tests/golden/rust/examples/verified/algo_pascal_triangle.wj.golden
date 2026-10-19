fn main() {
    let mut row: Vec<i64> = vec![1_i64];
    for _ in 0_i32..5_i32 {
        let strs: Vec<String> = row.iter().map(|x| x.to_string()).collect::<Vec<String>>();
        println!("{}", strs.join(" "));
        let mut next: Vec<i64> = vec![1_i64];
        for i in 0_usize..row.len() - 1_usize {
            next.push(row[i] + row[i + 1]);
        }
        next.push(1_i64);
        row = next;
    }
}

