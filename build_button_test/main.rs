use windjammer_ui::prelude::*;
use windjammer_ui::components::*;
use windjammer_ui::simple_vnode::{VNode, VAttr};
use wasm_bindgen::prelude::*;



#[inline]
#[wasm_bindgen]
pub fn start() {
    println!("🔘 Starting Button Test");
    let click_count = Signal::new(0);
    let render = move || {
        let click_count_handler = click_count.clone();
        let count_display = click_count.clone();
        Container::new().max_width("600px").child(Panel::new("Button Click Test".to_string()).child(Flex::new().direction(FlexDirection::Column).gap("20px").child(Text::new("Click the button below and check the browser console!".to_string())).child(Text::new(format!("Clicks so far: {}", count_display.get())).size(TextSize::Large)).child(Button::new("Click Me!".to_string()).variant(ButtonVariant::Primary).size(ButtonSize::Large).on_click(move || {
            let current = click_count_handler.get();
            let new_count = current + 1;
            click_count_handler.set(new_count);
            println!("🎉 Button clicked! Count: {}", new_count)
        })).child(Alert::info("Check the console AND watch the count update!".to_string())))).to_vnode()
    };
    println!("✅ UI created, mounting...");
    ReactiveApp::new("Button Test".to_string(), render).run();
    println!("✅ UI mounted! Click the button to test.")
}

fn main() {
    start()
}

