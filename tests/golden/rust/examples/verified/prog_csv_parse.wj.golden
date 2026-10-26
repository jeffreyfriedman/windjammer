#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]

struct Row {
    name: String,
    age: i64,
    city: String,
}

#[inline]
fn parse_line(line: &str) -> Result<Row, String> {
    let f: Vec<String> = line.split(",").map(|s| s.to_string()).map(|s| s.to_string()).collect::<Vec<String>>();
    if f.len() != 3_usize {
        return Err(String::from("wrong field count"));
    }
    let age = f[1].parse::<i64>().map_err(|_| "bad age")?;
    Ok(Row { name: f[0].clone(), age, city: f[2].clone() })
}

fn main() {
    let data: String = String::from("alice,30,paris\nbob,25,rome\ncarol,x,oslo");
    let mut rows: Vec<Row> = vec![];
    for line in data.lines() {
        match parse_line(&line) {
            Ok(r) => rows.push(r),
            Err(_) => println!("skipped bad line: {}", line),
        }
    }
    for r in &rows {
        println!("{} ({}) from {}", r.name, r.age, r.city);
    }
    let total: i64 = rows.iter().map(|r| r.age).sum();
    println!("average age: {}", total as f64 / rows.len() as f64);
}

