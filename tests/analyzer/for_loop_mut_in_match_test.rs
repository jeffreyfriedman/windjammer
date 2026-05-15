/// TDD Tests: For-loop iterator mutation detection inside if-let / match blocks
///
/// Bug: `if let` is desugared to `Statement::Match` by the parser, but
/// `statement_modifies_variable` didn't recurse into match arms. This caused
/// the codegen to emit `&self.entities` instead of `&mut self.entities` when
/// mutations happen inside `if let` blocks within for loops.
///
/// Example:
///   for entity in self.entities {
///     if let Some(vel) = entity.velocity {
///       entity.transform.x = entity.transform.x + vel.dx * dt  // mutation!
///     }
///   }
///
/// Expected: `for entity in &mut self.entities { ... }`
/// Actual (broken): `for entity in &self.entities { ... }`
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop_mut_borrow_when_mutation_inside_if_let() {
    let generated = test_utils::compile_single(
        r#"
@derive(Clone)
pub struct Velocity {
    pub dx: f32,
    pub dy: f32,
}

@derive(Clone)
pub struct Transform {
    pub x: f32,
    pub y: f32,
}

@derive(Clone)
pub struct Entity {
    pub transform: Transform,
    pub velocity: Option<Velocity>,
}

pub struct World {
    entities: Vec<Entity>,
}

impl World {
    pub fn apply_velocities(self, dt: f32) {
        for entity in self.entities {
            if let Some(vel) = entity.velocity {
                entity.transform.x = entity.transform.x + vel.dx * dt
                entity.transform.y = entity.transform.y + vel.dy * dt
            }
        }
    }
}
"#,
    );

    assert!(
        generated.contains("for entity in &mut self.entities"),
        "Expected `for entity in &mut self.entities` when loop body mutates entity \
         through nested field inside if-let block.\nGenerated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop_mut_borrow_when_mutation_inside_match() {
    let generated = test_utils::compile_single(
        r#"
@derive(Clone)
pub struct Item {
    pub count: i32,
    pub name: string,
}

pub struct Inventory {
    items: Vec<Item>,
}

impl Inventory {
    pub fn apply_bonus(self, bonus: i32) {
        for item in self.items {
            match item.name {
                "potion" => {
                    item.count = item.count + bonus
                },
                _ => {},
            }
        }
    }
}
"#,
    );

    assert!(
        generated.contains("for item in &mut self.items"),
        "Expected `for item in &mut self.items` when loop body mutates item \
         inside match arm.\nGenerated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop_immutable_borrow_when_no_mutation_in_match() {
    let generated = test_utils::compile_single(
        r#"
@derive(Clone)
pub struct Item {
    pub count: i32,
    pub name: string,
}

pub struct Container {
    items: Vec<Item>,
}

impl Container {
    pub fn total_count(self) -> i32 {
        let mut total = 0
        for item in self.items {
            match item.name {
                "potion" => {
                    total = total + item.count
                },
                _ => {},
            }
        }
        total
    }
}
"#,
    );

    assert!(
        !generated.contains("for item in &mut self.items"),
        "Expected immutable borrow `for item in &self.items` when loop body \
         does NOT mutate item inside match arm.\nGenerated:\n{}",
        generated
    );
}
