


use windjammer_game_framework::renderer::{Renderer, Color};
use windjammer_game_framework::input::{Input, Key, MouseButton};
use windjammer_game_framework::math::{Vec3, Mat4};
use windjammer_game_framework::ecs::*;
use windjammer_game_framework::game_app::GameApp;
use windjammer_ui::prelude::*;
use windjammer_ui::components::*;
use windjammer_ui::simple_vnode::{VNode, VAttr};




fn main() {
    let button = Button::new("Start Game".to_string()).variant(ButtonVariant::Primary);
    println!("UI Button created!");
    let position = Vec3::new(10.0, 20.0, 30.0);
    let color = Color::rgb(0.5, 0.8, 1.0);
    println!("Game types created!");
    println!("Position: ({}, {}, {})", position.x, position.y, position.z);
    println!("UI + Game framework integration works!")
}

