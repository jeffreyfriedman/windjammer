#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]

struct Task {
    name: String,
    pri: i64,
}

fn main() {
    let mut tasks = vec![Task { name: String::from("docs"), pri: 3_i64 }, Task { name: String::from("lint"), pri: 2_i64 }, Task { name: String::from("deploy"), pri: 1_i64 }, Task { name: String::from("review"), pri: 2_i64 }];
    tasks.sort_by(|a, b| a.pri.cmp(&b.pri).then(b.name.cmp(&a.name.clone())));
    for (i, t) in tasks.iter().enumerate() {
        println!("{}. {} (p{})", i + 1_usize, t.name, t.pri);
    }
}

