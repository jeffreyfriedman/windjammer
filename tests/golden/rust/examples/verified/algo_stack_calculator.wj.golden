
fn eval_rpn(expr: &str) -> Result<i64, String> {
    let mut stack: Vec<i64> = vec![];
    for tok in expr.split_whitespace() {
        if tok == "+" || tok == "-" || tok == "*" || tok == "/" {
            if stack.len() < 2_usize {
                return Err(String::from("stack underflow"));
            }
            let b = stack.pop().unwrap_or(0);
            let a = stack.pop().unwrap_or(0);
            let r = match tok.as_str() {
                "+" => a + b,
                "-" => a - b,
                "*" => a * b,
                _ => a / b,
            };
            stack.push(r)
        } else {
            stack.push(tok.parse::<i64>().unwrap_or(0_i64))
        }
    }
    Ok(stack[0])
}

fn main() {
    for e in ["2 3 4 * +", "15 7 1 1 + - /", "1 +"] {
        match eval_rpn(&e) {
            Ok(v) => println!("{}", v),
            Err(m) => println!("error: {}", m),
        }
    }
}

