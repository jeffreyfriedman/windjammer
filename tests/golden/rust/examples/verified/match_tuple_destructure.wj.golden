#[inline]
fn locate(p: (i64, i64)) -> String {
    match p {
        (0, 0) => String::from("origin"),
        (x, 0) => format!("on x axis at {}", x),
        (0, y) => format!("on y axis at {}", y),
        (x, y) => format!("at ({}, {})", x, y),
    }
}

fn main() {
    println!("{}", locate((0_i64, 0_i64)));
    println!("{}", locate((5_i64, 0_i64)));
    println!("{}", locate((0_i64, -2_i64)));
    println!("{}", locate((3_i64, 4_i64)));
}

