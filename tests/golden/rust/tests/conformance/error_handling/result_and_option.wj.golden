#[derive(Clone, Debug, PartialEq)]
enum ParseResult {
    Ok(i64),
    Err(String),
}

#[inline]
fn test_option_some() {
    let val: Option<i64> = Some(42_i64);
    match val {
        Some(v) => println!("[option_some] found: {}", v),
        None => println!("[option_some] not found"),
    }
}

#[inline]
fn test_option_none() {
    let val: Option<i64> = None;
    let result = match val {
        Some(v) => v,
        None => 0_i64,
    };
    println!("[option_none] not found: default={}", result);
}

#[inline]
fn describe_option(opt: Option<i64>) -> String {
    match opt {
        Some(v) => format!("value is {}", v),
        None => String::from("no value"),
    }
}

#[inline]
fn test_option_match() {
    println!("[option_match] Some(10) -> {}", describe_option(Some(10_i64)));
    println!("[option_match] None -> {}", describe_option(None));
}

#[inline]
fn double_option(opt: Option<i64>) -> Option<i64> {
    match opt {
        Some(v) => Some::<i64>(v * 2_i64),
        None => None,
    }
}

#[inline]
fn test_option_map() {
    let result = double_option(Some(5_i32));
    match result {
        Some(v) => println!("[option_map] Some(5) mapped -> {}", v),
        None => println!("[option_map] None"),
    }
}

#[inline]
fn lookup_name(id: i64) -> Option<String> {
    if id == 1_i64 {
        Some(String::from("Alice"))
    } else {
        if id == 2_i64 {
            Some(String::from("Bob"))
        } else {
            None
        }
    }
}

#[inline]
fn test_option_chain() {
    let name1: String = match lookup_name(1_i64) {
        Some(n) => n,
        None => String::from("Unknown"),
    };
    println!("[option_chain] lookup(1) -> found: {}", name1);
    let name2: String = match lookup_name(99_i64) {
        Some(n) => n,
        None => String::from("Unknown"),
    };
    println!("[option_chain] lookup(99) -> not found: {}", name2);
}

#[inline]
fn parse_int(s: &str) -> ParseResult {
    if s == "42" {
        ParseResult::Ok(42_i64)
    } else {
        if s == "100" {
            ParseResult::Ok(100_i64)
        } else {
            ParseResult::Err(String::from("invalid number"))
        }
    }
}

#[inline]
fn test_result_ok() {
    match parse_int("42") {
        ParseResult::Ok(v) => println!("[result_ok] parse(42) -> {}", v),
        ParseResult::Err(e) => println!("[result_ok] error: {}", e),
    }
}

#[inline]
fn test_result_err() {
    match parse_int("abc") {
        ParseResult::Ok(v) => println!("[result_err] parse(abc) -> {}", v),
        ParseResult::Err(e) => println!("[result_err] parse(abc) -> error: {}", e),
    }
}

#[inline]
fn test_result_match() {
    let results = vec!["100".to_string(), "xyz".to_string()];
    for s in results {
        match parse_int(&s) {
            ParseResult::Ok(v) => println!("[result_match] Ok({}) -> success: {}", v, v),
            ParseResult::Err(e) => println!("[result_match] Err -> failure: {}", e),
        }
    }
}

fn main() {
    test_option_some();
    test_option_none();
    test_option_match();
    test_option_map();
    test_option_chain();
    test_result_ok();
    test_result_err();
    test_result_match();
    println!("[error_all] PASSED");
}

