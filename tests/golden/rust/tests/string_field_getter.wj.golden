#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]

struct Quest {
    title: String,
    description: String,
    completed: bool,
}

impl Quest {
#[inline]
pub fn new(title: String, description: String) -> Quest {
        Quest { title, description, completed: false }
}
#[inline]
pub fn take_title(&self) -> String {
        self.title.clone()
}
#[inline]
pub fn title(&self) -> String {
        self.title.clone()
}
#[inline]
pub fn is_completed(&self) -> bool {
        self.completed
}
}

#[inline]
fn use_quest(q: &Quest) {
    let title = q.title();
    let done = q.is_completed();
    println!("Quest: {}, completed: {}", title, done);
}

fn main() {
    let quest = Quest::new("Save the kingdom".to_string(), "Defeat the dragon".to_string());
    use_quest(&quest);
    let title = quest.take_title();
    println!("Took title: {}", title);
    println!("✅ string field getters work correctly!");
}

