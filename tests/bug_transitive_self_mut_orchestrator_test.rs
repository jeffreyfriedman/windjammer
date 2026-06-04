#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

/// TDD Test: Orchestrator methods calling 3+ subsystem mutating methods
///
/// Bug: Game state orchestrator methods like `update()` only delegate to
/// subsystem methods (self.combat.tick(), self.hud.refresh(), self.spawner.step()).
/// None of these perform direct `self.field = ...` assignments in the orchestrator,
/// so the analyzer infers &self instead of &mut self.
///
/// This is the dominant pattern causing ~599 E0596 errors in breach-protocol.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_orchestrator_with_three_mutating_subsystems() {
    let source = r#"
pub struct CombatSystem {
    pub active_enemies: i32,
}

impl CombatSystem {
    pub fn tick(self, dt: f32) {
        self.active_enemies = self.active_enemies - 1
    }
}

pub struct SpawnManager {
    pub timer: f32,
}

impl SpawnManager {
    pub fn step(self, dt: f32) {
        self.timer = self.timer + dt
    }
}

pub struct HudDisplay {
    pub needs_redraw: bool,
}

impl HudDisplay {
    pub fn refresh(self) {
        self.needs_redraw = false
    }
}

pub struct GameState {
    pub combat: CombatSystem,
    pub spawner: SpawnManager,
    pub hud: HudDisplay,
}

impl GameState {
    pub fn update(self, dt: f32) {
        self.combat.tick(dt)
        self.spawner.step(dt)
        self.hud.refresh()
    }

    pub fn render(self) -> i32 {
        return self.combat.active_enemies
    }
}
"#;

    let generated = test_utils::compile_single_result(source)
        .expect("Windjammer compilation should succeed");

    assert!(
        generated.contains("fn update(&mut self"),
        "Orchestrator update() delegates to 3 mutating subsystems -- must be &mut self.\nGenerated:\n{}",
        generated
    );

    assert!(
        generated.contains("fn render(&self") || generated.contains("fn render(self"),
        "render() only reads -- should be &self or self (Copy types get self by value).\nGenerated:\n{}",
        generated
    );
    assert!(
        !generated.contains("fn render(&mut self"),
        "render() only reads -- must NOT be &mut self.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_nested_delegation_chain() {
    let source = r#"
pub struct Inner {
    pub count: i32,
}

impl Inner {
    pub fn increment(self) {
        self.count = self.count + 1
    }
}

pub struct Middle {
    pub inner: Inner,
}

impl Middle {
    pub fn process(self) {
        self.inner.increment()
    }
}

pub struct Outer {
    pub middle: Middle,
}

impl Outer {
    pub fn run(self) {
        self.middle.process()
    }
}
"#;

    let generated = test_utils::compile_single_result(source)
        .expect("Windjammer compilation should succeed");

    assert!(
        generated.contains("fn run(&mut self"),
        "run() -> middle.process() -> inner.increment() (mutates) -- must propagate &mut self.\nGenerated:\n{}",
        generated
    );

    assert!(
        generated.contains("fn process(&mut self"),
        "process() -> inner.increment() (mutates) -- must be &mut self.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_mixed_mut_and_readonly_subsystems() {
    let source = r#"
pub struct Config {
    pub name: String,
}

impl Config {
    pub fn get_name(self) -> String {
        return self.name.clone()
    }
}

pub struct Mutable {
    pub val: i32,
}

impl Mutable {
    pub fn change(self) {
        self.val = self.val + 1
    }
}

pub struct App {
    pub config: Config,
    pub state: Mutable,
}

impl App {
    pub fn run(self) {
        let _name = self.config.get_name()
        self.state.change()
    }
}
"#;

    let generated = test_utils::compile_single_result(source)
        .expect("Windjammer compilation should succeed");

    assert!(
        generated.contains("fn run(&mut self"),
        "run() calls self.state.change() which mutates -- must be &mut self even if config.get_name() is readonly.\nGenerated:\n{}",
        generated
    );
}
