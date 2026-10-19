#[derive(Clone, Debug, PartialEq, Copy)]
enum Light {
    Red,
    Green,
    Yellow,
}

#[inline]
fn action(l: Light) -> String {
    match l {
        Light::Red => String::from("stop"),
        Light::Green => String::from("go"),
        Light::Yellow => String::from("caution"),
    }
}

fn main() {
    println!("{}", action(Light::Red));
    println!("{}", action(Light::Green));
    println!("{}", action(Light::Yellow));
}

