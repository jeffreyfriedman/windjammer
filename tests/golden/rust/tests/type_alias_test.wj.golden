#[derive(Debug, Clone)]
#[repr(C)]
pub struct User {
    pub id: UserId,
    pub name: String,
}

pub type UserId = u32;

pub type QuestId = String;

pub type Position = (f32, f32);

#[inline]
pub fn get_user_id(user: User) -> UserId {
    user.id
}

fn main() {
    let user = User { id: 42, name: String::from("Alice") };
    let id: UserId = get_user_id(user);
    println!("User ID: {}", id);
    let quest: QuestId = "rescue_silas";
    println!("{}", process_quest(&quest));
}

#[inline]
pub fn process_quest(quest_id: &QuestId) -> String {
    format!("Processing quest: {}", quest_id)
}

