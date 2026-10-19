pub trait Speak {
    fn sound(&self) -> String;
    fn speak(&self, name: String) -> String {
        format!("{} says {}", name, self.sound())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Dog;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Cat;

impl Speak for Dog {
#[inline]
fn sound(&self) -> String {
        String::from("Woof")
}
}

impl Speak for Cat {
#[inline]
fn sound(&self) -> String {
        String::from("Meow")
}
}

fn main() {
    let d = Dog {  };
    let c = Cat {  };
    println!("{}", d.sound());
    println!("{}", c.sound());
    println!("{}", d.speak("Rex".to_string()));
}

