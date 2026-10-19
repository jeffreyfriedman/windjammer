#[derive(Clone, Debug, PartialEq)]
pub enum Speaker {
    Player,
    NPC(String),
    System,
}

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct Message {
    pub speaker: Speaker,
    pub text: String,
}

fn main() {
    let _msg = Message { speaker: Speaker::NPC(String::from("Alice")), text: String::from("Hello world") };
    println!("Message from NPC");
}

