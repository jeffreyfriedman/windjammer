use smallvec::{SmallVec, smallvec};

#[inline]
fn test_vec_literals() {
    let small: SmallVec<[_; 4]> = smallvec![1, 2, 3];
    let medium: SmallVec<[_; 8]> = smallvec![1, 2, 3, 4, 5];
    let large = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    println("small: {:?}", small);
    println("medium: {:?}", medium);
    println("large: {:?}", large)
}

#[inline]
fn test_range_collect() {
    let items: SmallVec<[_; 4]> = (0..3).collect().into();
    let more: SmallVec<[_; 8]> = (1..8).collect().into();
    println("items: {:?}", items);
    println("more: {:?}", more)
}

fn main() {
    test_vec_literals();
    test_range_collect()
}

