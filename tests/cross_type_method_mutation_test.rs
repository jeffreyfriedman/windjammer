/// TDD: Cross-type method mutation detection in multi-file compilation.
///
/// Bug: When file A defines PlayerState::use_energy(&mut self) and file B has
/// Ability::activate(self, player: PlayerState) calling player.use_energy(),
/// the step 2 analysis of file B (without global registry) can't find
/// use_energy's signature, so it incorrectly infers player as Owned instead
/// of MutBorrowed. The .wj.meta records Owned, and this stale ownership
/// propagates to call sites that auto-borrow based on the registry.
///
/// Root Cause: Step 2 per-file analysis has no cross-file signatures.
/// Step 3 re-analysis corrects the definition but may not update all
/// registry entries (especially module-qualified fallback entries).
/// The codegen's fallback lookup finds stale step 2 entries.
#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_single_file_method_mutation_detected() {
    let source = r#"
struct PlayerState {
    energy: f32,
    max_energy: f32,
}

impl PlayerState {
    pub fn use_energy(self, amount: f32) -> bool {
        if self.energy < amount {
            return false
        }
        self.energy = self.energy - amount
        true
    }
}

pub struct Ability {
    pub energy_cost: f32,
}

impl Ability {
    pub fn activate(self, player: PlayerState) -> bool {
        player.use_energy(self.energy_cost)
    }
}
"#;
    let compiled = test_utils::compile_single(source);
    assert!(
        compiled.contains("player: &mut PlayerState"),
        "player parameter should be &mut PlayerState because player.use_energy() mutates player.\n\
         Generated:\n{}",
        compiled
    );
}

#[test]
fn test_single_file_readonly_method_copy_type_is_owned() {
    let source = r#"
struct PlayerState {
    energy: f32,
    max_energy: f32,
}

impl PlayerState {
    pub fn get_energy(self) -> f32 {
        self.energy
    }
}

pub struct Display {
    pub label: f32,
}

impl Display {
    pub fn show_energy(self, player: PlayerState) -> f32 {
        player.get_energy()
    }
}
"#;
    let compiled = test_utils::compile_single(source);
    // PlayerState is Copy (all f32 fields), so read-only Copy params are Owned (by value)
    assert!(
        compiled.contains("player: PlayerState"),
        "player parameter should be PlayerState (Copy, by value) since only read.\n\
         Generated:\n{}",
        compiled
    );
}

#[test]
fn test_single_file_method_mutation_in_if_block() {
    let source = r#"
struct Resource {
    amount: f32,
}

impl Resource {
    pub fn consume(self, qty: f32) -> bool {
        if self.amount < qty {
            return false
        }
        self.amount = self.amount - qty
        true
    }
}

pub fn try_use(res: Resource, needed: f32) -> bool {
    if needed > 0.0 {
        res.consume(needed)
    } else {
        true
    }
}
"#;
    let compiled = test_utils::compile_single(source);
    assert!(
        compiled.contains("res: &mut Resource"),
        "res parameter should be &mut Resource because res.consume() mutates res.\n\
         Generated:\n{}",
        compiled
    );
}

#[test]
fn test_multi_file_cross_type_method_mutation_detected() {
    // This is the actual breach-protocol pattern:
    // player/state.wj defines PlayerState::use_energy (mutates self)
    // combat/abilities.wj defines Ability::activate(player: PlayerState) calling player.use_energy()
    // entry.wj calls self.ability.activate(self.player)
    //
    // The call site must generate &mut self.player, not &self.player
    let files = &[
        (
            "state.wj",
            r#"
pub struct PlayerState {
    pub energy: f32,
    pub max_energy: f32,
}

impl PlayerState {
    pub fn new() -> PlayerState {
        PlayerState { energy: 100.0, max_energy: 100.0 }
    }

    pub fn use_energy(self, amount: f32) -> bool {
        if self.energy < amount {
            return false
        }
        self.energy = self.energy - amount
        true
    }

    pub fn get_energy(self) -> f32 {
        self.energy
    }
}
"#,
        ),
        (
            "abilities.wj",
            r#"
use crate::state::PlayerState

pub struct Ability {
    pub energy_cost: f32,
}

impl Ability {
    pub fn new(cost: f32) -> Ability {
        Ability { energy_cost: cost }
    }

    pub fn activate(self, player: PlayerState) -> bool {
        player.use_energy(self.energy_cost)
    }
}
"#,
        ),
        (
            "game.wj",
            r#"
use crate::state::PlayerState
use crate::abilities::Ability

pub struct Game {
    pub player: PlayerState,
    pub dash: Ability,
}

impl Game {
    pub fn new() -> Game {
        Game {
            player: PlayerState::new(),
            dash: Ability::new(25.0),
        }
    }

    pub fn try_dash(self) -> bool {
        self.dash.activate(self.player)
    }
}
"#,
        ),
    ];

    let results = test_utils::compile_project(files);

    // Check the abilities.rs definition
    let abilities_rs = results
        .get("abilities.rs")
        .expect("abilities.rs should be generated");
    assert!(
        abilities_rs.contains("player: &mut PlayerState"),
        "Ability::activate should have player: &mut PlayerState in definition.\n\
         Generated abilities.rs:\n{}",
        abilities_rs
    );

    // Check the game.rs call site - this is the critical test
    let game_rs = results.get("game.rs").expect("game.rs should be generated");
    assert!(
        game_rs.contains("&mut self.player"),
        "Call site should pass &mut self.player (not &self.player).\n\
         Generated game.rs:\n{}",
        game_rs
    );
}

#[test]
fn test_multi_file_collision_correct_mut_borrow() {
    // Reproduces the breach-protocol bug: TWO types named Ability with activate()
    // in different modules, causing name collision in the signature registry.
    // combat/abilities.wj: Ability::activate(self, player: PlayerState) -> bool
    // rpg/abilities.wj:    Ability::activate(self) -> bool (no player param)
    // entry.wj: calls self.combat_ability.activate(self.player) → needs &mut self.player
    let files = &[
        (
            "state.wj",
            r#"
pub struct PlayerState {
    pub energy: f32,
    pub max_energy: f32,
}

impl PlayerState {
    pub fn new() -> PlayerState {
        PlayerState { energy: 100.0, max_energy: 100.0 }
    }

    pub fn use_energy(self, amount: f32) -> bool {
        if self.energy < amount {
            return false
        }
        self.energy = self.energy - amount
        true
    }
}
"#,
        ),
        (
            "combat_abilities.wj",
            r#"
use crate::state::PlayerState

pub struct Ability {
    pub energy_cost: f32,
    pub active: bool,
}

impl Ability {
    pub fn new(cost: f32) -> Ability {
        Ability { energy_cost: cost, active: false }
    }

    pub fn activate(self, player: PlayerState) -> bool {
        player.use_energy(self.energy_cost)
    }
}
"#,
        ),
        (
            "rpg_abilities.wj",
            r#"
pub struct Ability {
    pub level: i32,
    pub active: bool,
}

impl Ability {
    pub fn new() -> Ability {
        Ability { level: 1, active: false }
    }

    pub fn activate(self) -> bool {
        if self.level > 0 {
            self.active = true
            true
        } else {
            false
        }
    }
}
"#,
        ),
        (
            "entry.wj",
            r#"
use crate::state::PlayerState
use crate::combat_abilities::Ability

pub struct Game {
    pub player: PlayerState,
    pub dash: Ability,
}

impl Game {
    pub fn new() -> Game {
        Game {
            player: PlayerState::new(),
            dash: Ability::new(25.0),
        }
    }

    pub fn try_dash(self) -> bool {
        self.dash.activate(self.player)
    }
}
"#,
        ),
    ];

    let results = test_utils::compile_project(files);

    // Check combat_abilities.rs definition has correct signature
    let combat_rs = results
        .get("combat_abilities.rs")
        .expect("combat_abilities.rs should be generated");
    assert!(
        combat_rs.contains("player: &mut PlayerState"),
        "combat Ability::activate should have player: &mut PlayerState.\n\
         Generated combat_abilities.rs:\n{}",
        combat_rs
    );

    // Check entry.rs call site uses &mut self.player
    let entry_rs = results
        .get("entry.rs")
        .expect("entry.rs should be generated");
    assert!(
        entry_rs.contains("&mut self.player"),
        "Call site with name collision should pass &mut self.player.\n\
         Generated entry.rs:\n{}",
        entry_rs
    );
}
