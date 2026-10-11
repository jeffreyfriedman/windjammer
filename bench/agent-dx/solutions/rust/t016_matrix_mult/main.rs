use std::io::Read;

type Matrix = Vec<Vec<i64>>;

fn read_matrix<'a>(it: &mut impl Iterator<Item = &'a str>) -> Matrix {
    let r: usize = it.next().unwrap().parse().unwrap();
    let c: usize = it.next().unwrap().parse().unwrap();
    (0..r)
        .map(|_| (0..c).map(|_| it.next().unwrap().parse().unwrap()).collect())
        .collect()
}

fn mul(a: &Matrix, b: &Matrix) -> Matrix {
    let inner = b.len();
    let cols = b[0].len();
    a.iter()
        .map(|row| (0..cols).map(|j| (0..inner).map(|k| row[k] * b[k][j]).sum()).collect())
        .collect()
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();
    let k: usize = it.next().unwrap().parse().unwrap();
    for _ in 0..k {
        let a = read_matrix(&mut it);
        let b = read_matrix(&mut it);
        if a[0].len() != b.len() {
            println!("dimension mismatch");
        } else {
            for row in mul(&a, &b) {
                let v: Vec<String> = row.iter().map(|x| x.to_string()).collect();
                println!("{}", v.join(" "));
            }
        }
        println!("---");
    }
}
