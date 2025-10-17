# Windjammer UI Framework - Quick Start Guide

## Overview

Windjammer UI is a cross-platform framework for building apps and games. Write once, deploy to Web, Desktop, and Mobile!

## Installation

```bash
# Windjammer UI is included with Windjammer
cargo new myapp
cd myapp
cargo add windjammer-ui
```

## Building Your First UI Component

```windjammer
use windjammer_ui::prelude::*;

#[component]
struct Counter {
    count: i32,
}

impl Counter {
    fn render() -> VNode {
        VElement::new("div")
            .child(VElement::new("h1")
                .child(VText::new(format!("Count: {count}"))))
            .child(VElement::new("button")
                .child(VText::new("Increment")))
            .into()
    }
}

fn main() {
    let counter = Counter::new();
    println!("Rendered: {:?}", counter.render());
}
```

## Building Your First Game

```windjammer
use windjammer_ui::game::*;

#[game_entity]
struct Player {
    position: Vec2,
    velocity: Vec2,
}

impl GameEntity for Player {
    fn update(delta: f32) {
        position += velocity * delta;
    }
    
    fn id() -> EntityId {
        0
    }
}

fn main() {
    let mut player = Player {
        id: 0,
        position: Vec2::new(0.0, 0.0),
        velocity: Vec2::new(100.0, 0.0),
    };
    
    player.update(0.016); // One frame at 60 FPS
    println!("New position: {:?}", player.position);
}
```

## Platform Support

Windjammer UI works on:
- ✅ Web (JavaScript/WASM)
- ✅ Desktop (Tauri - macOS, Windows, Linux)
- ✅ Mobile (iOS, Android)

## Idiomatic Windjammer

Write clean code without Rust complexity:
- No `&self` or `&mut self` - auto-inferred
- No `&` in loops - auto-detected
- Direct field access (`position` not `self.position`)
- Format strings auto-borrow (`{score}` not `{:?}`)

## Examples

See the `examples/` directory:
- `counter.rs` - UI component example
- `simple_game.rs` - 2D platformer with physics
- `interactive_game.rs` - Full input handling

Run them with:
```bash
cargo run --example counter -p windjammer-ui
cargo run --example simple_game -p windjammer-ui
cargo run --example interactive_game -p windjammer-ui
```

## Learn More

- Design Document: `docs/design/windjammer-ui.md`
- ROADMAP: Future features and 3D support
- CHANGELOG: Detailed feature list

## Get Help

- GitHub Issues: Report bugs or request features
- Discord: Join our community
- Docs: https://docs.windjammer.dev

