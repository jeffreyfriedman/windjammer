compile_error!("missing boundary signature for `draw::draw_text`");
use crate::ui::draw;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Game {
    active: bool,
    selected: i32,
}










impl Game {
#[inline]
fn render(&self) {
        let _r = {
            if self.active {
                1.0_f64
            } else {
                0.6_f64
            }
        };
        let prefix: String = {
            if self.active {
                String::from(">> ")
            } else {
                String::from("   ")
            }
        };
        draw::draw_text("HELLO WORLD", 100.0_f64, 200.0_f64);
        draw::draw_text(prefix, 50.0_f64, 50.0_f64);
}
}

fn main() {
    let game = Game { active: true, selected: 0_i32 };
    game.render();
}

