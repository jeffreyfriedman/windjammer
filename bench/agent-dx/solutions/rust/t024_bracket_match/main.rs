use std::io::Read;

fn check(line: &str) -> Option<usize> {
    let mut stack: Vec<(char, usize)> = Vec::new();
    for (i, c) in line.chars().enumerate() {
        match c {
            '(' | '[' | '{' => stack.push((c, i + 1)),
            ')' | ']' | '}' => {
                let want = match c {
                    ')' => '(',
                    ']' => '[',
                    _ => '{',
                };
                match stack.pop() {
                    Some((o, _)) if o == want => {}
                    _ => return Some(i + 1),
                }
            }
            _ => {}
        }
    }
    stack.last().map(|&(_, i)| i)
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    for line in s.lines() {
        match check(line.trim_end_matches('\r')) {
            None => println!("ok"),
            Some(i) => println!("error {}", i),
        }
    }
}
