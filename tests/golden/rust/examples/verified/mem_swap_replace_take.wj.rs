fn main() {
    let mut v = vec![1_i32, 2_i32];
    v.swap(0_usize, 1_usize);
    println!("{:?}", v);
    let mut slot: Option<i64> = Some(5_i64);
    let taken = slot.take();
    println!("{:?} {:?}", taken, slot);
    let mut items: Vec<i64> = vec![1_i64, 2_i64];
    let old: Vec<i64> = items.clone();
    items = vec![];
    println!("{:?} {:?}", old, items);
}

