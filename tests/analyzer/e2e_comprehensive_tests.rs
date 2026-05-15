#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! Comprehensive End-to-End Compilation Tests
//!
//! These tests verify complete Windjammer programs compile to valid Rust,
//! testing realistic scenarios that exercise multiple compiler features.

#[path = "../common/test_utils.rs"]
mod test_utils;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

// ============================================================================
// COMPLETE PROGRAMS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_e2e_counter() {
    let code = r#"
@derive(Clone, Debug)
pub struct Counter {
    value: i32,
}

impl Counter {
    pub fn new() -> Counter {
        Counter { value: 0 }
    }
    
    pub fn increment(&mut self) {
        self.value += 1
    }
    
    pub fn decrement(&mut self) {
        self.value -= 1
    }
    
    pub fn get(&self) -> i32 {
        self.value
    }
    
    pub fn reset(&mut self) {
        self.value = 0
    }
}

pub fn test_counter() -> i32 {
    let mut c = Counter::new();
    c.increment();
    c.increment();
    c.increment();
    c.decrement();
    c.get()
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Counter program should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_e2e_stack() {
    let code = r#"
@derive(Clone, Debug)
pub struct Stack {
    items: Vec<i32>,
}

impl Stack {
    pub fn new() -> Stack {
        Stack { items: Vec::new() }
    }
    
    pub fn push(&mut self, item: i32) {
        self.items.push(item)
    }
    
    pub fn pop(&mut self) -> Option<i32> {
        self.items.pop()
    }
    
    pub fn peek(&self) -> Option<i32> {
        self.items.last().copied()
    }
    
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    
    pub fn len(&self) -> usize {
        self.items.len()
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Stack program should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_e2e_point_operations() {
    let code = r#"
@derive(Clone, Debug, Copy)
pub struct Point {
    x: f32,
    y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Point {
        Point { x: x, y: y }
    }
    
    pub fn origin() -> Point {
        Point { x: 0.0, y: 0.0 }
    }
    
    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.x += dx;
        self.y += dy
    }
    
    pub fn scale(&mut self, factor: f32) {
        self.x *= factor;
        self.y *= factor
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Point operations should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_e2e_linked_list_node() {
    // Simpler version without Box
    let code = r#"
@derive(Clone, Debug)
pub struct Node {
    value: i32,
    has_next: bool,
}

impl Node {
    pub fn new(value: i32) -> Node {
        Node { value: value, has_next: false }
    }
    
    pub fn value(&self) -> i32 {
        self.value
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Node should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_e2e_calculator() {
    let code = r#"
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

pub fn calculate(a: i32, b: i32, op: Operation) -> Option<i32> {
    match op {
        Operation::Add => Some(a + b),
        Operation::Subtract => Some(a - b),
        Operation::Multiply => Some(a * b),
        Operation::Divide => {
            if b == 0 {
                None
            } else {
                Some(a / b)
            }
        }
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Calculator should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_e2e_event_handler() {
    // Simpler event enum
    let code = r#"
@derive(Clone, Debug)
pub enum Event {
    Click,
    KeyPress,
    Resize,
}

@derive(Clone, Debug)
pub struct EventHandler {
    event_count: i32,
}

impl EventHandler {
    pub fn new() -> EventHandler {
        EventHandler { event_count: 0 }
    }
    
    pub fn handle(&mut self, _event: Event) {
        self.event_count += 1
    }
    
    pub fn count(&self) -> i32 {
        self.event_count
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Event handler should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_e2e_state_machine() {
    let code = r#"
@derive(Clone, Debug, PartialEq)
pub enum State {
    Idle,
    Running,
    Paused,
    Stopped,
}

@derive(Clone, Debug)
pub struct StateMachine {
    state: State,
}

impl StateMachine {
    pub fn new() -> StateMachine {
        StateMachine { state: State::Idle }
    }
    
    pub fn start(&mut self) {
        if self.state == State::Idle {
            self.state = State::Running
        }
    }
    
    pub fn pause(&mut self) {
        if self.state == State::Running {
            self.state = State::Paused
        }
    }
    
    pub fn resume(&mut self) {
        if self.state == State::Paused {
            self.state = State::Running
        }
    }
    
    pub fn stop(&mut self) {
        self.state = State::Stopped
    }
    
    pub fn is_running(&self) -> bool {
        self.state == State::Running
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "State machine should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_e2e_iterator_pipeline() {
    let code = r#"
pub fn process_numbers(numbers: &Vec<i32>) -> i32 {
    numbers
        .iter()
        .filter(|n| **n > 0)
        .map(|n| n * 2)
        .sum()
}

pub fn find_max(numbers: &Vec<i32>) -> Option<i32> {
    numbers.iter().max().copied()
}

pub fn count_positive(numbers: &Vec<i32>) -> usize {
    numbers.iter().filter(|n| **n > 0).count()
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Iterator pipeline should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_e2e_recursive_sum() {
    let code = r#"
pub fn sum_recursive(n: i32) -> i32 {
    if n <= 0 {
        0
    } else {
        n + sum_recursive(n - 1)
    }
}

pub fn factorial(n: i32) -> i32 {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "Recursive functions should compile. Error: {}",
        err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_e2e_builder_pattern() {
    let code = r#"
@derive(Clone, Debug, Default)
pub struct Config {
    width: i32,
    height: i32,
    title: string,
    fullscreen: bool,
}

impl Config {
    pub fn new() -> Config {
        Config::default()
    }
    
    pub fn width(&mut self, w: i32) -> &mut Config {
        self.width = w;
        self
    }
    
    pub fn height(&mut self, h: i32) -> &mut Config {
        self.height = h;
        self
    }
    
    pub fn fullscreen(&mut self, f: bool) -> &mut Config {
        self.fullscreen = f;
        self
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Builder pattern should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_e2e_generic_container() {
    let code = r#"
@derive(Clone, Debug)
pub struct Pair<T> {
    first: T,
    second: T,
}

impl<T: Clone> Pair<T> {
    pub fn new(first: T, second: T) -> Pair<T> {
        Pair { first: first, second: second }
    }
    
    pub fn swap(&mut self) {
        let temp = self.first.clone();
        self.first = self.second.clone();
        self.second = temp
    }
    
    pub fn get_first(&self) -> T {
        self.first.clone()
    }
    
    pub fn get_second(&self) -> T {
        self.second.clone()
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Generic container should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_e2e_game_entity() {
    let code = r#"
@derive(Clone, Debug)
pub struct Transform {
    x: f32,
    y: f32,
    rotation: f32,
    scale: f32,
}

@derive(Clone, Debug)
pub struct Entity {
    id: i32,
    active: bool,
}

impl Entity {
    pub fn new(id: i32) -> Entity {
        Entity {
            id: id,
            active: true,
        }
    }
    
    pub fn set_active(&mut self, active: bool) {
        self.active = active
    }
    
    pub fn is_active(&self) -> bool {
        self.active
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Game entity should compile. Error: {}", err);
}
