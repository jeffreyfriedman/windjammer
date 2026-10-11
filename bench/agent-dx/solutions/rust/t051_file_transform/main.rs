use std::collections::HashSet;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        println!("usage: transform INPUT OUTPUT MODE");
        std::process::exit(2);
    }
    let (input, output, mode) = (&args[0], &args[1], args[2].as_str());
    let text = match std::fs::read(input) {
        Ok(d) => String::from_utf8_lossy(&d).into_owned(),
        Err(_) => {
            println!("error: cannot read {}", input);
            std::process::exit(1);
        }
    };
    let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    match mode {
        "upper" => lines = lines.iter().map(|l| l.to_ascii_uppercase()).collect(),
        "reverse" => lines.reverse(),
        "sort" => lines.sort(),
        "dedupe" => {
            let mut seen = HashSet::new();
            lines.retain(|l| seen.insert(l.clone()));
        }
        "trim" => lines = lines.iter().map(|l| l.trim_matches(' ').to_string()).collect(),
        "number" => {
            lines = lines.iter().enumerate().map(|(i, l)| format!("{}:{}", i + 1, l)).collect()
        }
        _ => {
            println!("error: unknown mode {}", mode);
            std::process::exit(2);
        }
    }
    let body: String = lines.iter().map(|l| format!("{}\n", l)).collect();
    std::fs::write(output, &body).unwrap();
    println!("wrote {} lines to {}", lines.len(), output);
    print!("{}", std::fs::read_to_string(output).unwrap());
}
