#[inline]
fn bubble_sort(v: Vec<i64>) -> Vec<i64> {
    let mut a: Vec<i64> = v;
    let n = a.len();
    for i in 0_usize..n {
        for j in 0_usize..n - 1_usize - i {
            if a[j] > a[j + 1] {
                let _ = a.swap(j, (j + 1) as usize);
            }
        }
    }
    a
}

fn main() {
    let sorted: Vec<i64> = bubble_sort([64_i32, 34_i32, 25_i32, 12_i32, 22_i32, 11_i32, 90_i32]);
    println!("{:?}", sorted);
}

