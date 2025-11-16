//! Simple AI Behavior Tree System
//!
//! A simplified behavior tree for game AI that avoids complex borrowing issues.
//! For production use, consider using a mature library like `big-brain` or `behavior-tree`.

use std::collections::HashMap;

/// Simple blackboard for AI state
#[derive(Debug, Clone, Default)]
pub struct AIBlackboard {
    bools: HashMap<String, bool>,
    ints: HashMap<String, i32>,
    floats: HashMap<String, f32>,
}

impl AIBlackboard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_bool(&mut self, key: &str, value: bool) {
        self.bools.insert(key.to_string(), value);
    }

    pub fn get_bool(&self, key: &str) -> bool {
        self.bools.get(key).copied().unwrap_or(false)
    }

    pub fn set_int(&mut self, key: &str, value: i32) {
        self.ints.insert(key.to_string(), value);
    }

    pub fn get_int(&self, key: &str) -> i32 {
        self.ints.get(key).copied().unwrap_or(0)
    }

    pub fn set_float(&mut self, key: &str, value: f32) {
        self.floats.insert(key.to_string(), value);
    }

    pub fn get_float(&self, key: &str) -> f32 {
        self.floats.get(key).copied().unwrap_or(0.0)
    }
}

/// AI task status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AIStatus {
    Success,
    Failure,
    Running,
}

/// Simple AI task
pub trait AITask {
    fn execute(&mut self, blackboard: &mut AIBlackboard) -> AIStatus;
}

/// Sequence task (all must succeed)
pub struct SequenceTask {
    pub tasks: Vec<Box<dyn AITask>>,
    current: usize,
}

impl SequenceTask {
    pub fn new(tasks: Vec<Box<dyn AITask>>) -> Self {
        Self { tasks, current: 0 }
    }
}

impl AITask for SequenceTask {
    fn execute(&mut self, blackboard: &mut AIBlackboard) -> AIStatus {
        while self.current < self.tasks.len() {
            match self.tasks[self.current].execute(blackboard) {
                AIStatus::Success => self.current += 1,
                AIStatus::Failure => {
                    self.current = 0;
                    return AIStatus::Failure;
                }
                AIStatus::Running => return AIStatus::Running,
            }
        }
        self.current = 0;
        AIStatus::Success
    }
}

/// Selector task (first to succeed)
pub struct SelectorTask {
    pub tasks: Vec<Box<dyn AITask>>,
    current: usize,
}

impl SelectorTask {
    pub fn new(tasks: Vec<Box<dyn AITask>>) -> Self {
        Self { tasks, current: 0 }
    }
}

impl AITask for SelectorTask {
    fn execute(&mut self, blackboard: &mut AIBlackboard) -> AIStatus {
        while self.current < self.tasks.len() {
            match self.tasks[self.current].execute(blackboard) {
                AIStatus::Success => {
                    self.current = 0;
                    return AIStatus::Success;
                }
                AIStatus::Failure => self.current += 1,
                AIStatus::Running => return AIStatus::Running,
            }
        }
        self.current = 0;
        AIStatus::Failure
    }
}

/// Condition task
pub struct ConditionTask {
    pub key: String,
}

impl ConditionTask {
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
        }
    }
}

impl AITask for ConditionTask {
    fn execute(&mut self, blackboard: &mut AIBlackboard) -> AIStatus {
        if blackboard.get_bool(&self.key) {
            AIStatus::Success
        } else {
            AIStatus::Failure
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysSucceed;
    impl AITask for AlwaysSucceed {
        fn execute(&mut self, _blackboard: &mut AIBlackboard) -> AIStatus {
            AIStatus::Success
        }
    }

    struct AlwaysFail;
    impl AITask for AlwaysFail {
        fn execute(&mut self, _blackboard: &mut AIBlackboard) -> AIStatus {
            AIStatus::Failure
        }
    }

    #[test]
    fn test_blackboard() {
        let mut bb = AIBlackboard::new();
        bb.set_bool("test", true);
        assert_eq!(bb.get_bool("test"), true);
        bb.set_int("count", 42);
        assert_eq!(bb.get_int("count"), 42);
        bb.set_float("health", 100.0);
        assert_eq!(bb.get_float("health"), 100.0);
        println!("✅ AIBlackboard works");
    }

    #[test]
    fn test_sequence_success() {
        let mut seq = SequenceTask::new(vec![
            Box::new(AlwaysSucceed),
            Box::new(AlwaysSucceed),
        ]);
        let mut bb = AIBlackboard::new();
        assert_eq!(seq.execute(&mut bb), AIStatus::Success);
        println!("✅ Sequence success works");
    }

    #[test]
    fn test_sequence_failure() {
        let mut seq = SequenceTask::new(vec![
            Box::new(AlwaysSucceed),
            Box::new(AlwaysFail),
        ]);
        let mut bb = AIBlackboard::new();
        assert_eq!(seq.execute(&mut bb), AIStatus::Failure);
        println!("✅ Sequence failure works");
    }

    #[test]
    fn test_selector_success() {
        let mut sel = SelectorTask::new(vec![
            Box::new(AlwaysFail),
            Box::new(AlwaysSucceed),
        ]);
        let mut bb = AIBlackboard::new();
        assert_eq!(sel.execute(&mut bb), AIStatus::Success);
        println!("✅ Selector success works");
    }

    #[test]
    fn test_selector_failure() {
        let mut sel = SelectorTask::new(vec![
            Box::new(AlwaysFail),
            Box::new(AlwaysFail),
        ]);
        let mut bb = AIBlackboard::new();
        assert_eq!(sel.execute(&mut bb), AIStatus::Failure);
        println!("✅ Selector failure works");
    }

    #[test]
    fn test_condition() {
        let mut cond = ConditionTask::new("flag");
        let mut bb = AIBlackboard::new();
        
        bb.set_bool("flag", false);
        assert_eq!(cond.execute(&mut bb), AIStatus::Failure);
        
        bb.set_bool("flag", true);
        assert_eq!(cond.execute(&mut bb), AIStatus::Success);
        
        println!("✅ Condition works");
    }
}

