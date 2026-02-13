//! Interpreter bug-fix and coverage-gap tests — discovered by conformance dogfooding.

/// Parse source and run through the interpreter, capturing output.
fn interpret(source: &str) -> Result<String, String> {
    let mut lexer = windjammer::lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = windjammer::parser::Parser::new_with_source(
        tokens,
        "test.wj".to_string(),
        source.to_string(),
    );
    let program = parser.parse().map_err(|e| format!("Parse error: {}", e))?;
    let mut interp = windjammer::interpreter::Interpreter::new_capturing();
    interp.run(&program)?;
    Ok(interp.get_output())
}

// ==========================================
// Bug: Nested enum pattern destructuring
// Result::Ok(Maybe::Some(v)) should bind v=42
// ==========================================

#[test]
fn test_nested_enum_destructuring() {
    let output = interpret(
        r#"
enum Maybe {
    Some(int),
    None,
}

enum Result {
    Ok(Maybe),
    Err(string),
}

fn extract(r: Result) -> int {
    match r {
        Result::Ok(Maybe::Some(v)) => v,
        Result::Ok(Maybe::None) => 0,
        Result::Err(_) => -1,
    }
}

fn main() {
    println("{}", extract(Result::Ok(Maybe::Some(42))))
    println("{}", extract(Result::Ok(Maybe::None)))
    println("{}", extract(Result::Err("oops")))
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "42\n0\n-1");
}

// ==========================================
// Bug: Match guard (n if n > 0)
// ==========================================

#[test]
fn test_match_guard() {
    let output = interpret(
        r#"
fn classify(x: int) -> string {
    match x {
        n if n > 0 => "positive",
        0 => "zero",
        _ => "negative",
    }
}

fn main() {
    println("{}", classify(5))
    println("{}", classify(0))
    println("{}", classify(-3))
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "positive\nzero\nnegative");
}

// ==========================================
// Bug: Enum method via match self
// ==========================================

#[test]
fn test_enum_self_match() {
    let output = interpret(
        r#"
enum Shape {
    Circle(int),
    Square(int),
    Point,
}

impl Shape {
    fn area(self) -> int {
        match self {
            Shape::Circle(r) => 314 * r * r / 100,
            Shape::Square(s) => s * s,
            Shape::Point => 0,
        }
    }
}

fn main() {
    let c = Shape::Circle(10)
    let s = Shape::Square(5)
    println("{}", c.area())
    println("{}", s.area())
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "314\n25");
}

// ==========================================
// Bug: For over vec![] literal
// ==========================================

#[test]
fn test_for_over_vec_literal() {
    let output = interpret(
        r#"
fn main() {
    for x in vec![5, 0, -3] {
        println("{}", x)
    }
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "5\n0\n-3");
}

// ==========================================
// Bug: String .len() method
// ==========================================

#[test]
fn test_string_len() {
    let output = interpret(
        r#"
fn main() {
    let s = "hello"
    println("{}", s.len())
    let e = ""
    println("{}", e.len())
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "5\n0");
}

// ==========================================
// Bug: Vec .len() in interpolation
// ==========================================

#[test]
fn test_vec_len_interp() {
    let output = interpret(
        r#"
fn main() {
    let v = vec![1, 2, 3]
    println("len={}", v.len())
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "len=3");
}

// ==========================================
// Bug: Enum in match as let expression
// ==========================================

#[test]
fn test_match_enum_as_expression() {
    let output = interpret(
        r#"
enum Color { Red, Green, Blue }

fn main() {
    let c = Color::Green
    let name = match c {
        Color::Red => "red",
        Color::Green => "green",
        Color::Blue => "blue",
    }
    println("{}", name)
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "green");
}

// ==========================================
// Bug: Chained struct field access via println
// ==========================================

#[test]
fn test_struct_field_in_format() {
    let output = interpret(
        r#"
struct Point { x: int, y: int }

fn main() {
    let p = Point { x: 10, y: 20 }
    println("x={}, y={}", p.x, p.y)
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "x=10, y=20");
}

// ==========================================
// Coverage gap: Continue statement
// ==========================================

#[test]
fn test_continue_while() {
    let output = interpret(
        r#"
fn main() {
    let mut i = 0
    while i < 6 {
        i += 1
        if i % 2 == 0 {
            continue
        }
        println("{}", i)
    }
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "1\n3\n5");
}

#[test]
fn test_continue_for_range() {
    let output = interpret(
        r#"
fn main() {
    for i in 0..8 {
        if i % 3 == 0 {
            continue
        }
        println("{}", i)
    }
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "1\n2\n4\n5\n7");
}
