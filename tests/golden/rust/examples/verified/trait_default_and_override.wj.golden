pub trait Greeter {
    fn name(&self) -> String;
    fn greet(&self) -> String {
        format!("Hi, I am {}", self.name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Robot;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]

struct Person {
    n: String,
}

impl Greeter for Robot {
#[inline]
fn name(&self) -> String {
        String::from("Robot")
}
}

impl Greeter for Person {
#[inline]
fn name(&self) -> String {
        self.n.clone()
}
#[inline]
fn greet(&self) -> String {
        format!("Hello, I am {}", self.n)
}
}

fn main() {
    println!("{}", Robot {  }.greet());
    println!("{}", Person { n: String::from("Ann") }.greet());
}

