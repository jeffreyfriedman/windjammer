use windjammer_ui::prelude::*;
use windjammer_ui::components::*;
use windjammer_ui::simple_vnode::{VNode, VAttr};
use wasm_bindgen::prelude::*;



struct EditorDemoState {
    console_output: windjammer_ui::reactivity::Signal<String>,
    button_clicks: windjammer_ui::reactivity::Signal<i32>,
}

impl EditorDemoState {
#[inline]
fn new() -> EditorDemoState {
        EditorDemoState { console_output: Signal::new("Welcome to Windjammer Game Editor!
Click the buttons to see them work!
".to_string()), button_clicks: Signal::new(0) }
}
#[inline]
fn log(&self, message: &String) {
        let current = self.console_output.get();
        self.console_output.set(format!("{}{}{}", current, message, "
"))
}
#[inline]
fn increment_clicks(&self) {
        let current = self.button_clicks.get();
        self.button_clicks.set(current + 1)
}
}

#[inline]
fn render_editor(state: &EditorDemoState) -> Container {
    let clicks = state.button_clicks.get();
    let console_text = state.console_output.get();
    Container::new().max_width("100%").child(render_header(&state.clone())).child(render_toolbar(&state.clone())).child(render_main_area(&state.clone())).child(render_console(&state.clone())).child(render_status_bar(clicks))
}

#[inline]
fn render_header(state: &EditorDemoState) -> Panel {
    Panel::new("🎮 Windjammer Game Editor".to_string()).child(Text::new("Interactive Demo - All buttons are functional!".to_string()))
}

#[inline]
fn render_toolbar(state: &EditorDemoState) -> Panel {
    let state_new = state.clone();
    let state_open = state.clone();
    let state_save = state.clone();
    let state_run = state.clone();
    let state_stop = state.clone();
    Panel::new("Toolbar".to_string()).child(Flex::new().direction(FlexDirection::Row).gap("8px").child(Button::new("New Project".to_string()).variant(ButtonVariant::Primary).on_click(move || {
        state_new.increment_clicks();
        state_new.log("📁 Creating new project...".to_string())
    })).child(Button::new("Open".to_string()).variant(ButtonVariant::Secondary).on_click(move || {
        state_open.increment_clicks();
        state_open.log("📂 Opening project...".to_string())
    })).child(Button::new("Save".to_string()).variant(ButtonVariant::Secondary).on_click(move || {
        state_save.increment_clicks();
        state_save.log("💾 Saving files...".to_string())
    })).child(Button::new("Run".to_string()).variant(ButtonVariant::Primary).on_click(move || {
        state_run.increment_clicks();
        state_run.log("▶️  Running game...".to_string())
    })).child(Button::new("Stop".to_string()).variant(ButtonVariant::Danger).on_click(move || {
        state_stop.increment_clicks();
        state_stop.log("⏹️  Game stopped".to_string())
    })))
}

#[inline]
fn render_main_area(state: &EditorDemoState) -> Flex {
    Flex::new().direction(FlexDirection::Row).gap("10px").child(render_file_panel()).child(render_editor_panel()).child(render_preview_panel())
}

#[inline]
fn render_file_panel() -> Panel {
    Panel::new("📁 Files".to_string()).child(Text::new("src/
  main.wj
  game.wj
assets/
  player.png".to_string()))
}

#[inline]
fn render_editor_panel() -> Panel {
    Panel::new("✏️ Editor".to_string()).child(Text::new("// main.wj
use std::game::*

fn main() {
    println!(\"Hello Game!\")
}".to_string()))
}

#[inline]
fn render_preview_panel() -> Panel {
    Panel::new("👁️ Preview".to_string()).child(Text::new("Game preview will appear here
when you click Run".to_string()))
}

#[inline]
fn render_console(state: &EditorDemoState) -> Panel {
    let output = state.console_output.get();
    Panel::new("🖥️ Console".to_string()).child(Text::new(output))
}

#[inline]
fn render_status_bar(clicks: i32) -> Panel {
    Panel::new("Status".to_string()).child(Text::new(format!("✅ Editor ready | Total button clicks: {}", clicks)).size(TextSize::Small))
}

#[inline]
#[wasm_bindgen]
pub fn start() {
    println!("🎮 Starting Windjammer Game Editor Demo");
    let state = EditorDemoState::new();
    let render = move || {
        render_editor(&state.clone()).to_vnode()
    };
    ReactiveApp::new("Windjammer Game Editor".to_string(), render).run()
}

fn main() {
    start()
}

