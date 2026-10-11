use std::io::Read;

trait Animal {
    fn name(&self) -> &str;
    fn kind(&self) -> &str;
    fn sound(&self) -> &str;
    fn legs(&self) -> u32;
    fn describe(&self) -> String {
        format!("{} the {} says {} on {} legs", self.name(), self.kind(), self.sound(), self.legs())
    }
}

struct Basic {
    name: String,
    kind: &'static str,
    sound: &'static str,
    legs: u32,
}
impl Animal for Basic {
    fn name(&self) -> &str { &self.name }
    fn kind(&self) -> &str { self.kind }
    fn sound(&self) -> &str { self.sound }
    fn legs(&self) -> u32 { self.legs }
}

struct Cat(String);
impl Animal for Cat {
    fn name(&self) -> &str { &self.0 }
    fn kind(&self) -> &str { "cat" }
    fn sound(&self) -> &str { "meow" }
    fn legs(&self) -> u32 { 4 }
    fn describe(&self) -> String {
        format!("{} the cat purrs softly", self.0)
    }
}

fn make(kind: &str, name: &str) -> Option<Box<dyn Animal>> {
    let b = |kind, sound, legs| -> Option<Box<dyn Animal>> {
        Some(Box::new(Basic { name: name.to_string(), kind, sound, legs }))
    };
    match kind {
        "dog" => b("dog", "woof", 4),
        "cat" => Some(Box::new(Cat(name.to_string()))),
        "duck" => b("duck", "quack", 2),
        "snake" => b("snake", "hiss", 0),
        "spider" => b("spider", "...", 8),
        _ => None,
    }
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut total = 0;
    for line in s.lines() {
        let mut it = line.split_whitespace();
        let Some(kind) = it.next() else { continue };
        let name = it.next().unwrap_or("");
        match make(kind, name) {
            Some(a) => {
                println!("{}", a.describe());
                total += a.legs();
            }
            None => println!("unknown kind {}", kind),
        }
    }
    println!("legs total={}", total);
}
