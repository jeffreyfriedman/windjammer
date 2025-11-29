#[inline]
pub fn counter() -> i64 {
    let mut count = 0;
    count += 1;
    count += 1;
    count
}

#[inline]
pub fn accumulate(values: Vec<i64>) -> i64 {
    let mut sum = 0;
    for value in values {
        sum += value;
    }
    sum
}

