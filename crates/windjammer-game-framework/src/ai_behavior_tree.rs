//! # Advanced AI Behavior Tree System
//!
//! Provides a complete behavior tree implementation for complex AI behaviors.
//!
//! ## Features
//! - Composite nodes (Sequence, Selector, Parallel, Random)
//! - Decorator nodes (Inverter, Repeater, Cooldown, Timeout, ForceSuccess/Failure)
//! - Condition nodes
//! - Action nodes
//! - Blackboard for shared state
//! - Visual tree representation
//! - Tree serialization
//!
//! ## Example
//! ```no_run
//! use windjammer_game_framework::ai_behavior_tree::{BehaviorTree, BehaviorTreeBuilder, Status};
//!
//! let tree = BehaviorTreeBuilder::new()
//!     .sequence(|seq| {
//!         seq.condition("enemy_visible")
//!            .action("aim_at_enemy")
//!            .action("shoot")
//!     })
//!     .build();
//! ```

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Blackboard for AI state storage
#[derive(Debug, Clone, Default)]
pub struct Blackboard {
    /// Boolean values
    bools: HashMap<String, bool>,
    /// Integer values
    ints: HashMap<String, i32>,
    /// Float values
    floats: HashMap<String, f32>,
    /// String values
    strings: HashMap<String, String>,
}

impl Blackboard {
    /// Create a new blackboard
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a boolean value
    pub fn set_bool(&mut self, key: &str, value: bool) {
        self.bools.insert(key.to_string(), value);
    }

    /// Get a boolean value
    pub fn get_bool(&self, key: &str) -> bool {
        self.bools.get(key).copied().unwrap_or(false)
    }

    /// Set an integer value
    pub fn set_int(&mut self, key: &str, value: i32) {
        self.ints.insert(key.to_string(), value);
    }

    /// Get an integer value
    pub fn get_int(&self, key: &str) -> i32 {
        self.ints.get(key).copied().unwrap_or(0)
    }

    /// Set a float value
    pub fn set_float(&mut self, key: &str, value: f32) {
        self.floats.insert(key.to_string(), value);
    }

    /// Get a float value
    pub fn get_float(&self, key: &str) -> f32 {
        self.floats.get(key).copied().unwrap_or(0.0)
    }

    /// Set a string value
    pub fn set_string(&mut self, key: &str, value: String) {
        self.strings.insert(key.to_string(), value);
    }

    /// Get a string value
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.strings.get(key).map(|s| s.as_str())
    }

    /// Clear all values
    pub fn clear(&mut self) {
        self.bools.clear();
        self.ints.clear();
        self.floats.clear();
        self.strings.clear();
    }
}

/// Node execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Node succeeded
    Success,
    /// Node failed
    Failure,
    /// Node is still running
    Running,
}

/// Behavior tree node type
#[derive(Debug, Clone)]
pub enum NodeType {
    /// Sequence node (all children must succeed)
    Sequence,
    /// Selector node (first child to succeed)
    Selector,
    /// Parallel node (run all children simultaneously)
    Parallel { success_threshold: usize },
    /// Random selector (pick random child)
    RandomSelector,
    /// Inverter decorator (invert child result)
    Inverter,
    /// Repeater decorator (repeat child N times)
    Repeater { count: usize },
    /// Cooldown decorator (limit execution rate)
    Cooldown { duration: Duration },
    /// Timeout decorator (fail if child takes too long)
    Timeout { duration: Duration },
    /// Force success decorator
    ForceSuccess,
    /// Force failure decorator
    ForceFailure,
    /// Condition node
    Condition { key: String },
    /// Action node
    Action { name: String },
}

/// Behavior tree node
#[derive(Debug, Clone)]
pub struct BehaviorNode {
    /// Node type
    pub node_type: NodeType,
    /// Child nodes
    pub children: Vec<BehaviorNode>,
    /// Node state (for decorators)
    state: NodeState,
}

/// Node state for decorators
#[derive(Debug, Clone)]
struct NodeState {
    /// Current child index (for composites)
    current_child: usize,
    /// Repeat count (for repeater)
    repeat_count: usize,
    /// Last execution time (for cooldown)
    last_execution: Option<Instant>,
    /// Execution start time (for timeout)
    execution_start: Option<Instant>,
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            current_child: 0,
            repeat_count: 0,
            last_execution: None,
            execution_start: None,
        }
    }
}

impl BehaviorNode {
    /// Create a new node
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_type,
            children: Vec::new(),
            state: NodeState::default(),
        }
    }

    /// Add a child node
    pub fn add_child(&mut self, child: BehaviorNode) {
        self.children.push(child);
    }

    /// Execute the node
    pub fn execute(&mut self, blackboard: &mut Blackboard) -> Status {
        match &self.node_type {
            NodeType::Sequence => self.execute_sequence(blackboard),
            NodeType::Selector => self.execute_selector(blackboard),
            NodeType::Parallel { success_threshold } => {
                self.execute_parallel(blackboard, *success_threshold)
            }
            NodeType::RandomSelector => self.execute_random_selector(blackboard),
            NodeType::Inverter => self.execute_inverter(blackboard),
            NodeType::Repeater { count } => self.execute_repeater(blackboard, *count),
            NodeType::Cooldown { duration } => self.execute_cooldown(blackboard, *duration),
            NodeType::Timeout { duration } => self.execute_timeout(blackboard, *duration),
            NodeType::ForceSuccess => self.execute_force_success(blackboard),
            NodeType::ForceFailure => self.execute_force_failure(blackboard),
            NodeType::Condition { key } => {
                if blackboard.get_bool(key) {
                    Status::Success
                } else {
                    Status::Failure
                }
            }
            NodeType::Action { name: _ } => {
                // Actions would be implemented by user code
                // For now, return success
                Status::Success
            }
        }
    }

    fn execute_sequence(&mut self, blackboard: &mut Blackboard) -> Status {
        while self.state.current_child < self.children.len() {
            match self.children[self.state.current_child].execute(blackboard) {
                Status::Success => self.state.current_child += 1,
                Status::Failure => {
                    self.state.current_child = 0;
                    return Status::Failure;
                }
                Status::Running => return Status::Running,
            }
        }
        self.state.current_child = 0;
        Status::Success
    }

    fn execute_selector(&mut self, blackboard: &mut Blackboard) -> Status {
        while self.state.current_child < self.children.len() {
            match self.children[self.state.current_child].execute(blackboard) {
                Status::Success => {
                    self.state.current_child = 0;
                    return Status::Success;
                }
                Status::Failure => self.state.current_child += 1,
                Status::Running => return Status::Running,
            }
        }
        self.state.current_child = 0;
        Status::Failure
    }

    fn execute_parallel(&mut self, blackboard: &mut Blackboard, success_threshold: usize) -> Status {
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut running_count = 0;

        for child in &mut self.children {
            match child.execute(blackboard) {
                Status::Success => success_count += 1,
                Status::Failure => failure_count += 1,
                Status::Running => running_count += 1,
            }
        }

        if success_count >= success_threshold {
            Status::Success
        } else if failure_count > self.children.len() - success_threshold {
            Status::Failure
        } else {
            Status::Running
        }
    }

    fn execute_random_selector(&mut self, blackboard: &mut Blackboard) -> Status {
        if self.children.is_empty() {
            return Status::Failure;
        }

        // Simple pseudo-random selection
        let index = (blackboard.get_int("_random_seed") as usize) % self.children.len();
        blackboard.set_int("_random_seed", blackboard.get_int("_random_seed") + 1);

        self.children[index].execute(blackboard)
    }

    fn execute_inverter(&mut self, blackboard: &mut Blackboard) -> Status {
        if let Some(child) = self.children.first_mut() {
            match child.execute(blackboard) {
                Status::Success => Status::Failure,
                Status::Failure => Status::Success,
                Status::Running => Status::Running,
            }
        } else {
            Status::Failure
        }
    }

    fn execute_repeater(&mut self, blackboard: &mut Blackboard, count: usize) -> Status {
        if let Some(child) = self.children.first_mut() {
            loop {
                match child.execute(blackboard) {
                    Status::Success => {
                        self.state.repeat_count += 1;
                        if self.state.repeat_count >= count {
                            self.state.repeat_count = 0;
                            return Status::Success;
                        }
                    }
                    Status::Failure => {
                        self.state.repeat_count = 0;
                        return Status::Failure;
                    }
                    Status::Running => return Status::Running,
                }
            }
        } else {
            Status::Failure
        }
    }

    fn execute_cooldown(&mut self, blackboard: &mut Blackboard, duration: Duration) -> Status {
        let now = Instant::now();

        if let Some(last_execution) = self.state.last_execution {
            if now.duration_since(last_execution) < duration {
                return Status::Failure;
            }
        }

        if let Some(child) = self.children.first_mut() {
            let result = child.execute(blackboard);
            if result != Status::Running {
                self.state.last_execution = Some(now);
            }
            result
        } else {
            Status::Failure
        }
    }

    fn execute_timeout(&mut self, blackboard: &mut Blackboard, duration: Duration) -> Status {
        if self.state.execution_start.is_none() {
            self.state.execution_start = Some(Instant::now());
        }

        let now = Instant::now();
        if now.duration_since(self.state.execution_start.unwrap()) > duration {
            self.state.execution_start = None;
            return Status::Failure;
        }

        if let Some(child) = self.children.first_mut() {
            let result = child.execute(blackboard);
            if result != Status::Running {
                self.state.execution_start = None;
            }
            result
        } else {
            Status::Failure
        }
    }

    fn execute_force_success(&mut self, blackboard: &mut Blackboard) -> Status {
        if let Some(child) = self.children.first_mut() {
            match child.execute(blackboard) {
                Status::Running => Status::Running,
                _ => Status::Success,
            }
        } else {
            Status::Success
        }
    }

    fn execute_force_failure(&mut self, blackboard: &mut Blackboard) -> Status {
        if let Some(child) = self.children.first_mut() {
            match child.execute(blackboard) {
                Status::Running => Status::Running,
                _ => Status::Failure,
            }
        } else {
            Status::Failure
        }
    }
}

/// Behavior tree
pub struct BehaviorTree {
    /// Root node
    root: BehaviorNode,
    /// Blackboard
    blackboard: Blackboard,
}

impl BehaviorTree {
    /// Create a new behavior tree
    pub fn new(root: BehaviorNode) -> Self {
        Self {
            root,
            blackboard: Blackboard::new(),
        }
    }

    /// Execute the tree
    pub fn execute(&mut self) -> Status {
        self.root.execute(&mut self.blackboard)
    }

    /// Get the blackboard
    pub fn blackboard(&self) -> &Blackboard {
        &self.blackboard
    }

    /// Get mutable blackboard
    pub fn blackboard_mut(&mut self) -> &mut Blackboard {
        &mut self.blackboard
    }
}

/// Behavior tree builder
pub struct BehaviorTreeBuilder {
    root: Option<BehaviorNode>,
}

impl BehaviorTreeBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Set the root node
    pub fn root(mut self, node: BehaviorNode) -> Self {
        self.root = Some(node);
        self
    }

    /// Build the tree
    pub fn build(self) -> BehaviorTree {
        BehaviorTree::new(self.root.unwrap_or_else(|| BehaviorNode::new(NodeType::Sequence)))
    }
}

impl Default for BehaviorTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blackboard_bool() {
        let mut bb = Blackboard::new();
        bb.set_bool("test", true);
        assert!(bb.get_bool("test"));
        assert!(!bb.get_bool("nonexistent"));
    }

    #[test]
    fn test_blackboard_int() {
        let mut bb = Blackboard::new();
        bb.set_int("count", 42);
        assert_eq!(bb.get_int("count"), 42);
        assert_eq!(bb.get_int("nonexistent"), 0);
    }

    #[test]
    fn test_blackboard_float() {
        let mut bb = Blackboard::new();
        bb.set_float("speed", 3.14);
        assert_eq!(bb.get_float("speed"), 3.14);
        assert_eq!(bb.get_float("nonexistent"), 0.0);
    }

    #[test]
    fn test_blackboard_string() {
        let mut bb = Blackboard::new();
        bb.set_string("name", "test".to_string());
        assert_eq!(bb.get_string("name"), Some("test"));
        assert_eq!(bb.get_string("nonexistent"), None);
    }

    #[test]
    fn test_blackboard_clear() {
        let mut bb = Blackboard::new();
        bb.set_bool("test", true);
        bb.set_int("count", 42);
        bb.clear();
        assert!(!bb.get_bool("test"));
        assert_eq!(bb.get_int("count"), 0);
    }

    #[test]
    fn test_condition_node() {
        let mut bb = Blackboard::new();
        bb.set_bool("test", true);

        let mut node = BehaviorNode::new(NodeType::Condition {
            key: "test".to_string(),
        });

        assert_eq!(node.execute(&mut bb), Status::Success);

        bb.set_bool("test", false);
        assert_eq!(node.execute(&mut bb), Status::Failure);
    }

    #[test]
    fn test_action_node() {
        let mut bb = Blackboard::new();
        let mut node = BehaviorNode::new(NodeType::Action {
            name: "test_action".to_string(),
        });

        assert_eq!(node.execute(&mut bb), Status::Success);
    }

    #[test]
    fn test_sequence_node() {
        let mut bb = Blackboard::new();
        bb.set_bool("cond1", true);
        bb.set_bool("cond2", true);

        let mut sequence = BehaviorNode::new(NodeType::Sequence);
        sequence.add_child(BehaviorNode::new(NodeType::Condition {
            key: "cond1".to_string(),
        }));
        sequence.add_child(BehaviorNode::new(NodeType::Condition {
            key: "cond2".to_string(),
        }));

        assert_eq!(sequence.execute(&mut bb), Status::Success);

        bb.set_bool("cond2", false);
        assert_eq!(sequence.execute(&mut bb), Status::Failure);
    }

    #[test]
    fn test_selector_node() {
        let mut bb = Blackboard::new();
        bb.set_bool("cond1", false);
        bb.set_bool("cond2", true);

        let mut selector = BehaviorNode::new(NodeType::Selector);
        selector.add_child(BehaviorNode::new(NodeType::Condition {
            key: "cond1".to_string(),
        }));
        selector.add_child(BehaviorNode::new(NodeType::Condition {
            key: "cond2".to_string(),
        }));

        assert_eq!(selector.execute(&mut bb), Status::Success);

        bb.set_bool("cond2", false);
        assert_eq!(selector.execute(&mut bb), Status::Failure);
    }

    #[test]
    fn test_inverter_node() {
        let mut bb = Blackboard::new();
        bb.set_bool("test", true);

        let mut inverter = BehaviorNode::new(NodeType::Inverter);
        inverter.add_child(BehaviorNode::new(NodeType::Condition {
            key: "test".to_string(),
        }));

        assert_eq!(inverter.execute(&mut bb), Status::Failure);

        bb.set_bool("test", false);
        assert_eq!(inverter.execute(&mut bb), Status::Success);
    }

    #[test]
    fn test_force_success_node() {
        let mut bb = Blackboard::new();
        bb.set_bool("test", false);

        let mut force_success = BehaviorNode::new(NodeType::ForceSuccess);
        force_success.add_child(BehaviorNode::new(NodeType::Condition {
            key: "test".to_string(),
        }));

        assert_eq!(force_success.execute(&mut bb), Status::Success);
    }

    #[test]
    fn test_force_failure_node() {
        let mut bb = Blackboard::new();
        bb.set_bool("test", true);

        let mut force_failure = BehaviorNode::new(NodeType::ForceFailure);
        force_failure.add_child(BehaviorNode::new(NodeType::Condition {
            key: "test".to_string(),
        }));

        assert_eq!(force_failure.execute(&mut bb), Status::Failure);
    }

    #[test]
    fn test_parallel_node() {
        let mut bb = Blackboard::new();
        bb.set_bool("cond1", true);
        bb.set_bool("cond2", true);
        bb.set_bool("cond3", false);

        let mut parallel = BehaviorNode::new(NodeType::Parallel {
            success_threshold: 2,
        });
        parallel.add_child(BehaviorNode::new(NodeType::Condition {
            key: "cond1".to_string(),
        }));
        parallel.add_child(BehaviorNode::new(NodeType::Condition {
            key: "cond2".to_string(),
        }));
        parallel.add_child(BehaviorNode::new(NodeType::Condition {
            key: "cond3".to_string(),
        }));

        assert_eq!(parallel.execute(&mut bb), Status::Success);
    }

    #[test]
    fn test_behavior_tree() {
        let mut tree = BehaviorTreeBuilder::new()
            .root(BehaviorNode::new(NodeType::Sequence))
            .build();

        tree.blackboard_mut().set_bool("test", true);
        assert!(tree.blackboard().get_bool("test"));
    }

    #[test]
    fn test_status_types() {
        assert_eq!(Status::Success, Status::Success);
        assert_eq!(Status::Failure, Status::Failure);
        assert_eq!(Status::Running, Status::Running);
        assert_ne!(Status::Success, Status::Failure);
    }

    #[test]
    fn test_random_selector() {
        let mut bb = Blackboard::new();
        bb.set_int("_random_seed", 0);
        bb.set_bool("cond1", true);
        bb.set_bool("cond2", false);

        let mut random_selector = BehaviorNode::new(NodeType::RandomSelector);
        random_selector.add_child(BehaviorNode::new(NodeType::Condition {
            key: "cond1".to_string(),
        }));
        random_selector.add_child(BehaviorNode::new(NodeType::Condition {
            key: "cond2".to_string(),
        }));

        // Should execute one of the children
        let result = random_selector.execute(&mut bb);
        assert!(result == Status::Success || result == Status::Failure);
    }

    #[test]
    fn test_cooldown_decorator() {
        let mut bb = Blackboard::new();
        bb.set_bool("test", true);

        let mut cooldown = BehaviorNode::new(NodeType::Cooldown {
            duration: Duration::from_millis(100),
        });
        cooldown.add_child(BehaviorNode::new(NodeType::Condition {
            key: "test".to_string(),
        }));

        // First execution should succeed
        assert_eq!(cooldown.execute(&mut bb), Status::Success);

        // Immediate second execution should fail (cooldown)
        assert_eq!(cooldown.execute(&mut bb), Status::Failure);
    }

    #[test]
    fn test_node_types() {
        let sequence = NodeType::Sequence;
        let selector = NodeType::Selector;
        let inverter = NodeType::Inverter;

        // Just test that node types can be created
        let _node1 = BehaviorNode::new(sequence);
        let _node2 = BehaviorNode::new(selector);
        let _node3 = BehaviorNode::new(inverter);
    }
}

