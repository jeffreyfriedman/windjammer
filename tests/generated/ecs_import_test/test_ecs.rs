


use entity::Entity;
use components::Transform;

#[derive(Debug, Clone, Default)]
pub struct World {
    pub entities: Vec<Entity>,
}

