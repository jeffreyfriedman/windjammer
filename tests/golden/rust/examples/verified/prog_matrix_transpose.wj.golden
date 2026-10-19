#[inline]
fn transpose(m: &Vec<Vec<i64>>) -> Vec<Vec<i64>> {
    let rows = m.len();
    let cols = m[0].len();
    let mut out: Vec<Vec<i64>> = vec![];
    for c in 0_usize..cols {
        let mut row: Vec<i64> = vec![];
        for r in 0_usize..rows {
            row.push(m[r][c]);
        }
        out.push(row);
    }
    out
}

fn main() {
    for row in transpose(&[[1_i64, 2_i64, 3_i64], [4_i64, 5_i64, 6_i64]]) {
        let s: Vec<String> = row.iter().map(|x| x.to_string()).collect::<Vec<String>>();
        println!("{}", s.join(" "));
    }
}

