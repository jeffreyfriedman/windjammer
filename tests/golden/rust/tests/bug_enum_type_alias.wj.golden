#[derive(Clone, Debug)]
enum DialogueConsequence {
    CompleteQuest(QuestId),
    FailQuest(QuestId),
}

type QuestId = String;

#[inline]
fn create_consequence() -> DialogueConsequence {
    DialogueConsequence::CompleteQuest(String::from("rescue_mission"))
}

fn main() {
    let _c = create_consequence();
    println!("Created consequence");
}

