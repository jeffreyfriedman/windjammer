use windjammer_runtime::test::*;
#[test]

fn test_if_statement() {
    let x = 5_i64;
    let result: String = {
        if x > 3_i64 {
            String::from("greater")
        } else {
            String::from("less")
        }
    };
    assert!(result == "greater");
}

#[test]

fn test_if_else() {
    let x = 2_i64;
    let result: String = {
        if x > 3_i64 {
            String::from("greater")
        } else {
            String::from("less")
        }
    };
    assert!(result == "less");
}

#[test]

fn test_while_loop() {
    let mut count = 0_i64;
    while count < 5_i64 {
        count += 1;
    }
    assert!(count == 5_i64);
}

#[test]

fn test_for_loop() {
    let mut sum = 0_i64;
    for i in 0_i64..5_i64 {
        sum += i as i64;
    }
    assert!(sum == 10_i64);
}

#[test]

fn test_match_statement() {
    let x = 2_i32;
    let result: String = match x {
        1 => "one",
        2 => "two",
        3 => "three",
        _ => "other",
    };
    assert!(result == "two");
}

#[test]

fn test_match_with_default() {
    let x = 99_i32;
    let result: String = match x {
        1 => "one",
        2 => "two",
        _ => "other",
    };
    assert!(result == "other");
}

#[test]

fn test_nested_if() {
    let x = 5_i64;
    let y = 10_i64;
    let result: String = {
        if x > 3_i64 {
            if y > 8_i64 {
                String::from("both")
            } else {
                String::from("x only")
            }
        } else {
            String::from("neither")
        }
    };
    assert!(result == "both");
}

