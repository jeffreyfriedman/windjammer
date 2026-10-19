#[derive(Clone, Debug, PartialEq, Copy)]
enum Condition {
    ThresholdCheck(i32),
}

impl Condition {
#[inline]
pub fn check(&self) -> bool {
        match *self {
            Condition::ThresholdCheck(threshold) => {
                return get_value() > threshold;
            },
        }
}
}

#[inline]
fn get_value() -> i32 {
    42_i32
}

fn main() {
    let cond = Condition::ThresholdCheck(30_i32);
    let result = cond.check();
    println!("Result: {}", result);
}

