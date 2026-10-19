#[derive(Clone, Debug, PartialEq, Copy)]
enum Condition {
    ThresholdCheck(i32),
}

#[inline]
fn get_value() -> i32 {
    42_i32
}

#[inline]
fn check_condition(cond: Condition) -> bool {
    match cond {
        Condition::ThresholdCheck(threshold) => {
            return get_value() > threshold;
        },
    }
}

fn main() {
    let cond = Condition::ThresholdCheck(30_i32);
    let result = check_condition(cond);
    println!("Result: {}", result);
}

