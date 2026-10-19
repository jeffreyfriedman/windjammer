#[derive(Clone, Debug, PartialEq)]
pub enum Speaker {
    Player,
    NPC(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Text(String),
    Error(String),
}

#[inline]
pub fn test_enum_string_literal() {
    let speaker = Speaker::NPC(String::from("Silas Crane"));
    match speaker {
        Speaker::Player => assert!(false),
        Speaker::NPC(name) => {
            assert_eq!(name, "Silas Crane");
        },
    }
}

#[inline]
pub fn test_enum_variants_multiple() {
    let msg1 = Message::Text(String::from("Hello"));
    let _msg2 = Message::Error(String::from("Failed"));
    match msg1 {
        Message::Text(s) => assert_eq!(s, "Hello"),
        Message::Error(_) => assert!(false),
    }
}

