use std::io::Read;

fn eval(line: &str) -> Result<i64, String> {
    let mut nums = line.split(" / ").map(|t| t.trim().parse::<i64>().unwrap());
    let mut acc = nums.next().unwrap();
    for (i, d) in nums.enumerate() {
        let step = i + 1;
        if d == 0 {
            return Err(format!("error: division by zero at step {}", step));
        }
        acc = acc
            .checked_div(d)
            .ok_or_else(|| format!("error: overflow at step {}", step))?;
    }
    Ok(acc)
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    for line in s.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match eval(line) {
            Ok(v) => println!("{}", v),
            Err(e) => println!("{}", e),
        }
    }
}
