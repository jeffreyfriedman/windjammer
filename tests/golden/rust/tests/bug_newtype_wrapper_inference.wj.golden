#[derive(Clone, Debug)]
enum DialogueConsequence {
    SetFlag(String),
    CompleteQuest(QuestId),
    UnlockQuest(QuestId),
}

#[derive(Debug, Clone, Default)]

struct DialogueState {
    completed_quests: Vec<QuestId>,
}

impl DialogueState {
#[inline]
fn is_quest_completed(&self, quest_id: &QuestId) -> bool {
        let mut i: usize = 0_usize;
        while i < self.completed_quests.len() {
            if self.completed_quests[i] == quest_id {
                return true;
            }
            i += 1;
        }
        false
}
#[inline]
fn check_with_ref(&self, quest_id: &QuestId) -> bool {
        self.is_quest_completed(quest_id)
}
}

type QuestId = String;

fn main() {
    let _consequence = DialogueConsequence::CompleteQuest(String::from("rescue_silas"));
    let state = DialogueState { completed_quests: vec![] };
    let quest: String = String::from("test_quest");
    let is_complete = state.check_with_ref(&quest);
    println!("Quest complete: {}", is_complete);
}

