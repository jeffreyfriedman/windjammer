//! AI Behavior Tree System
//!
//! Provides a flexible behavior tree implementation for AAA game AI.
//!
//! ## Features
//! - Composite nodes (Sequence, Selector, Parallel)
//! - Decorator nodes (Inverter, Repeater, Conditional)
//! - Action & Condition leaf nodes
//! - Blackboard for shared data
//! - Reusable subtrees

use std::collections::HashMap;

/// Behavior tree
#[derive(Debug, Clone)]
pub struct BehaviorTree {
    /// Root node
    pub root: Box<BehaviorNode>,
    /// Blackboard (shared data)
    pub blackboard: Blackboard,
}

/// Blackboard for sharing data between nodes
#[derive(Debug, Clone)]
pub struct Blackboard {
    /// Data storage
    data: HashMap<String, BlackboardValue>,
}

/// Blackboard value types
#[derive(Debug, Clone, PartialEq)]
pub enum BlackboardValue {
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
}

/// Behavior tree node
#[derive(Debug, Clone)]
pub enum BehaviorNode {
    /// Composite nodes
    Sequence(Vec<BehaviorNode>),
    Selector(Vec<BehaviorNode>),
    Parallel {
        children: Vec<BehaviorNode>,
        success_threshold: usize,
    },

    /// Decorator nodes
    Inverter(Box<BehaviorNode>),
    Repeater {
        child: Box<BehaviorNode>,
        count: Option<usize>, // None = infinite
    },
    Conditional {
        condition: String, // Blackboard key
        child: Box<BehaviorNode>,
    },

    /// Leaf nodes
    Action(String),
    Condition(String),
}

/// Node execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeStatus {
    Success,
    Failure,
    Running,
}

/// Behavior tree context (for execution)
#[derive(Debug, Clone)]
pub struct BehaviorContext {
    /// Current node execution states
    node_states: HashMap<usize, NodeStatus>,
    /// Repeat counters for repeater nodes
    repeat_counters: HashMap<usize, usize>,
}

impl BehaviorTree {
    /// Create a new behavior tree
    pub fn new(root: BehaviorNode) -> Self {
        Self {
            root: Box::new(root),
            blackboard: Blackboard::new(),
        }
    }

    /// Execute the behavior tree
    pub fn tick(&mut self, context: &mut BehaviorContext) -> NodeStatus {
        // We need to work around Rust's borrow checker here
        // In a production system, you'd use Rc/RefCell or arena allocation
        let status = match &*self.root {
            BehaviorNode::Sequence(children) => self.execute_sequence(children, context, 0),
            BehaviorNode::Selector(children) => self.execute_selector(children, context, 0),
            BehaviorNode::Parallel { children, success_threshold } => {
                self.execute_parallel(children, *success_threshold, context, 0)
            }
            BehaviorNode::Inverter(child) => {
                self.execute_inverter_root(child, context, 0)
            }
            BehaviorNode::Repeater { child, count } => {
                self.execute_repeater_root(child, *count, context, 0)
            }
            BehaviorNode::Conditional { condition, child } => {
                self.execute_conditional_root(condition, child, context, 0)
            }
            BehaviorNode::Action(action) => self.execute_action(action),
            BehaviorNode::Condition(condition) => self.execute_condition(condition),
        };
        status
    }

    /// Execute a node (recursive helper)
    fn execute_node(
        &mut self,
        node: &BehaviorNode,
        context: &mut BehaviorContext,
        node_id: usize,
    ) -> NodeStatus {
        match node {
            BehaviorNode::Sequence(children) => self.execute_sequence(children, context, node_id),
            BehaviorNode::Selector(children) => self.execute_selector(children, context, node_id),
            BehaviorNode::Parallel {
                children,
                success_threshold,
            } => self.execute_parallel(children, *success_threshold, context, node_id),
            BehaviorNode::Inverter(child) => self.execute_inverter(child, context, node_id),
            BehaviorNode::Repeater { child, count } => {
                self.execute_repeater(child, *count, context, node_id)
            }
            BehaviorNode::Conditional { condition, child } => {
                self.execute_conditional(condition, child, context, node_id)
            }
            BehaviorNode::Action(action) => self.execute_action(action),
            BehaviorNode::Condition(condition) => self.execute_condition(condition),
        }
    }

    /// Execute sequence node (all children must succeed)
    fn execute_sequence(
        &mut self,
        children: &[BehaviorNode],
        context: &mut BehaviorContext,
        node_id: usize,
    ) -> NodeStatus {
        for (i, child) in children.iter().enumerate() {
            let status = self.execute_node(child, context, node_id * 1000 + i);
            match status {
                NodeStatus::Failure => return NodeStatus::Failure,
                NodeStatus::Running => return NodeStatus::Running,
                NodeStatus::Success => continue,
            }
        }
        NodeStatus::Success
    }

    /// Execute selector node (first child to succeed)
    fn execute_selector(
        &mut self,
        children: &[BehaviorNode],
        context: &mut BehaviorContext,
        node_id: usize,
    ) -> NodeStatus {
        for (i, child) in children.iter().enumerate() {
            let status = self.execute_node(child, context, node_id * 1000 + i);
            match status {
                NodeStatus::Success => return NodeStatus::Success,
                NodeStatus::Running => return NodeStatus::Running,
                NodeStatus::Failure => continue,
            }
        }
        NodeStatus::Failure
    }

    /// Execute parallel node (run all children, succeed if threshold met)
    fn execute_parallel(
        &mut self,
        children: &[BehaviorNode],
        success_threshold: usize,
        context: &mut BehaviorContext,
        node_id: usize,
    ) -> NodeStatus {
        let mut success_count = 0;
        let mut running_count = 0;

        for (i, child) in children.iter().enumerate() {
            let status = self.execute_node(child, context, node_id * 1000 + i);
            match status {
                NodeStatus::Success => success_count += 1,
                NodeStatus::Running => running_count += 1,
                NodeStatus::Failure => {}
            }
        }

        if success_count >= success_threshold {
            NodeStatus::Success
        } else if running_count > 0 {
            NodeStatus::Running
        } else {
            NodeStatus::Failure
        }
    }

    /// Execute inverter node (invert child result)
    fn execute_inverter(
        &mut self,
        child: &BehaviorNode,
        context: &mut BehaviorContext,
        node_id: usize,
    ) -> NodeStatus {
        let status = self.execute_node(child, context, node_id * 1000);
        match status {
            NodeStatus::Success => NodeStatus::Failure,
            NodeStatus::Failure => NodeStatus::Success,
            NodeStatus::Running => NodeStatus::Running,
        }
    }

    /// Execute inverter for root node
    fn execute_inverter_root(
        &mut self,
        child: &BehaviorNode,
        context: &mut BehaviorContext,
        node_id: usize,
    ) -> NodeStatus {
        self.execute_inverter(child, context, node_id)
    }

    /// Execute repeater node (repeat child N times or infinitely)
    fn execute_repeater(
        &mut self,
        child: &BehaviorNode,
        count: Option<usize>,
        context: &mut BehaviorContext,
        node_id: usize,
    ) -> NodeStatus {
        let current_count = context.repeat_counters.get(&node_id).copied().unwrap_or(0);

        if let Some(max_count) = count {
            if current_count >= max_count {
                context.repeat_counters.remove(&node_id);
                return NodeStatus::Success;
            }
        }

        let status = self.execute_node(child, context, node_id * 1000);

        match status {
            NodeStatus::Success | NodeStatus::Failure => {
                context.repeat_counters.insert(node_id, current_count + 1);
                NodeStatus::Running // Continue repeating
            }
            NodeStatus::Running => NodeStatus::Running,
        }
    }

    /// Execute repeater for root node
    fn execute_repeater_root(
        &mut self,
        child: &BehaviorNode,
        count: Option<usize>,
        context: &mut BehaviorContext,
        node_id: usize,
    ) -> NodeStatus {
        self.execute_repeater(child, count, context, node_id)
    }

    /// Execute conditional node (only run child if condition is true)
    fn execute_conditional(
        &mut self,
        condition: &str,
        child: &BehaviorNode,
        context: &mut BehaviorContext,
        node_id: usize,
    ) -> NodeStatus {
        if self.blackboard.get_bool(condition) {
            self.execute_node(child, context, node_id * 1000)
        } else {
            NodeStatus::Failure
        }
    }

    /// Execute conditional for root node
    fn execute_conditional_root(
        &mut self,
        condition: &str,
        child: &BehaviorNode,
        context: &mut BehaviorContext,
        node_id: usize,
    ) -> NodeStatus {
        self.execute_conditional(condition, child, context, node_id)
    }

    /// Execute action node (placeholder - should be overridden)
    fn execute_action(&mut self, _action: &str) -> NodeStatus {
        // In a real implementation, this would call a registered action handler
        NodeStatus::Success
    }

    /// Execute condition node (placeholder - should be overridden)
    fn execute_condition(&mut self, condition: &str) -> NodeStatus {
        if self.blackboard.get_bool(condition) {
            NodeStatus::Success
        } else {
            NodeStatus::Failure
        }
    }
}

impl Blackboard {
    /// Create a new blackboard
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Set a boolean value
    pub fn set_bool(&mut self, key: &str, value: bool) {
        self.data.insert(key.to_string(), BlackboardValue::Bool(value));
    }

    /// Get a boolean value
    pub fn get_bool(&self, key: &str) -> bool {
        match self.data.get(key) {
            Some(BlackboardValue::Bool(v)) => *v,
            _ => false,
        }
    }

    /// Set an integer value
    pub fn set_int(&mut self, key: &str, value: i32) {
        self.data.insert(key.to_string(), BlackboardValue::Int(value));
    }

    /// Get an integer value
    pub fn get_int(&self, key: &str) -> i32 {
        match self.data.get(key) {
            Some(BlackboardValue::Int(v)) => *v,
            _ => 0,
        }
    }

    /// Set a float value
    pub fn set_float(&mut self, key: &str, value: f32) {
        self.data.insert(key.to_string(), BlackboardValue::Float(value));
    }

    /// Get a float value
    pub fn get_float(&self, key: &str) -> f32 {
        match self.data.get(key) {
            Some(BlackboardValue::Float(v)) => *v,
            _ => 0.0,
        }
    }

    /// Set a string value
    pub fn set_string(&mut self, key: &str, value: String) {
        self.data.insert(key.to_string(), BlackboardValue::String(value));
    }

    /// Get a string value
    pub fn get_string(&self, key: &str) -> String {
        match self.data.get(key) {
            Some(BlackboardValue::String(v)) => v.clone(),
            _ => String::new(),
        }
    }

    /// Check if key exists
    pub fn has_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Remove a key
    pub fn remove(&mut self, key: &str) {
        self.data.remove(key);
    }

    /// Clear all data
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

impl Default for Blackboard {
    fn default() -> Self {
        Self::new()
    }
}

impl BehaviorContext {
    /// Create a new behavior context
    pub fn new() -> Self {
        Self {
            node_states: HashMap::new(),
            repeat_counters: HashMap::new(),
        }
    }

    /// Reset the context
    pub fn reset(&mut self) {
        self.node_states.clear();
        self.repeat_counters.clear();
    }
}

impl Default for BehaviorContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blackboard_creation() {
        let blackboard = Blackboard::new();
        assert!(!blackboard.has_key("test"));
        println!("✅ Blackboard created");
    }

    #[test]
    fn test_blackboard_bool() {
        let mut blackboard = Blackboard::new();
        blackboard.set_bool("flag", true);
        assert_eq!(blackboard.get_bool("flag"), true);
        println!("✅ Blackboard bool works");
    }

    #[test]
    fn test_blackboard_int() {
        let mut blackboard = Blackboard::new();
        blackboard.set_int("count", 42);
        assert_eq!(blackboard.get_int("count"), 42);
        println!("✅ Blackboard int works");
    }

    #[test]
    fn test_blackboard_float() {
        let mut blackboard = Blackboard::new();
        blackboard.set_float("health", 75.5);
        assert_eq!(blackboard.get_float("health"), 75.5);
        println!("✅ Blackboard float works");
    }

    #[test]
    fn test_sequence_success() {
        let tree = BehaviorTree::new(BehaviorNode::Sequence(vec![
            BehaviorNode::Condition("cond1".to_string()),
            BehaviorNode::Condition("cond2".to_string()),
        ]));
        
        let mut tree = tree;
        tree.blackboard.set_bool("cond1", true);
        tree.blackboard.set_bool("cond2", true);
        
        let mut context = BehaviorContext::new();
        let status = tree.tick(&mut context);
        
        assert_eq!(status, NodeStatus::Success);
        println!("✅ Sequence success works");
    }

    #[test]
    fn test_sequence_failure() {
        let tree = BehaviorTree::new(BehaviorNode::Sequence(vec![
            BehaviorNode::Condition("cond1".to_string()),
            BehaviorNode::Condition("cond2".to_string()),
        ]));
        
        let mut tree = tree;
        tree.blackboard.set_bool("cond1", true);
        tree.blackboard.set_bool("cond2", false);
        
        let mut context = BehaviorContext::new();
        let status = tree.tick(&mut context);
        
        assert_eq!(status, NodeStatus::Failure);
        println!("✅ Sequence failure works");
    }

    #[test]
    fn test_selector_success() {
        let tree = BehaviorTree::new(BehaviorNode::Selector(vec![
            BehaviorNode::Condition("cond1".to_string()),
            BehaviorNode::Condition("cond2".to_string()),
        ]));
        
        let mut tree = tree;
        tree.blackboard.set_bool("cond1", false);
        tree.blackboard.set_bool("cond2", true);
        
        let mut context = BehaviorContext::new();
        let status = tree.tick(&mut context);
        
        assert_eq!(status, NodeStatus::Success);
        println!("✅ Selector success works");
    }

    #[test]
    fn test_inverter() {
        let tree = BehaviorTree::new(BehaviorNode::Inverter(Box::new(
            BehaviorNode::Condition("flag".to_string()),
        )));
        
        let mut tree = tree;
        tree.blackboard.set_bool("flag", false);
        
        let mut context = BehaviorContext::new();
        let status = tree.tick(&mut context);
        
        assert_eq!(status, NodeStatus::Success);
        println!("✅ Inverter works");
    }
}

