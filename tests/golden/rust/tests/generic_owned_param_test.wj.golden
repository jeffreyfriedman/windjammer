pub trait GameState {
    fn tick(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct MyGame {
    pub frame: i32,
}
impl MyGame {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(4);
        __bytes.extend_from_slice(&self.frame.to_ne_bytes());
        __bytes
    }
}


impl GameState for MyGame {
#[inline]
fn tick(&mut self) {
        self.frame += 1;
}
}

extern "C" {
    pub fn process_state<S: GameState>(state: &mut S);
}

#[inline]
pub fn run_game<G: GameState>(mut game: G) {
    (unsafe { process_state(&mut game) });
}

#[inline]
pub fn test() {
    let game = MyGame { frame: 0_i32 };
    run_game(game);
}

