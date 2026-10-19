fn main() {
    let v = vec![1_i64, 2_i64, 3_i64, 4_i64, 5_i64, 6_i64];
    let (ev, od): (Vec<i64>, Vec<i64>) = v.iter().partition(|x| *x % 2_i64 == 0_i64);
    println!("{:?} {:?}", ev, od);
    let pairs = vec![(1_i32, "a"), (2_i32, "b"), (3_i32, "c")];
    let (ns, ls): (Vec<i64>, Vec<String>) = pairs.iter().cloned().unzip();
    println!("{:?} [{}]", ns, ls.join(", "));
}

