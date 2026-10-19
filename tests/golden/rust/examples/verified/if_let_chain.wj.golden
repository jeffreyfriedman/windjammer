#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]

struct User {
    name: String,
    email: Option<String>,
}

fn main() {
    let u = User { name: String::from("Ann"), email: None };
    if let name = u.name {
        let _ = println!("name = {}", name);
    }
    if let Some(e) = u.email {
        println!("email = {}", e);
    } else {
        println!("no email");
    }
    let r: Result<i64, String> = Err(String::from("boom"));
    if let Err(m) = r {
        let _ = println!("error: {}", m);
    }
}

