use windjammer_ui::prelude::*;
use windjammer_ui::components::*;
use windjammer_ui::simple_vnode::{VNode, VAttr};
use wasm_bindgen::prelude::*;



#[inline]
#[wasm_bindgen]
pub fn start() {
    println!("🔢 Starting Reactive Counter");
    let count = Signal::new(0);
    let render_count = count.clone();
    let button_count = count.clone();
    let render = move || {
        let display_count = render_count.clone();
        let text_count = render_count.clone();
        let dec_count = button_count.clone();
        let reset_count = button_count.clone();
        let inc_count = button_count.clone();
        Container::new().max_width("600px").child(Panel::new("Interactive Counter".to_string()).child(Flex::new().direction(FlexDirection::Column).gap("20px").child(Text::new(format!("Count: {}", display_count.get())).size(TextSize::XLarge)).child(Flex::new().direction(FlexDirection::Row).gap("10px").child(Button::new("- Decrement".to_string()).variant(ButtonVariant::Secondary).on_click(move || {
            let current = dec_count.get();
            dec_count.set(current - 1);
            println!("Decremented to: {}", current - 1)
        })).child(Button::new("Reset".to_string()).variant(ButtonVariant::Danger).on_click(move || {
            reset_count.set(0);
            println!("Reset to 0")
        })).child(Button::new("+ Increment".to_string()).variant(ButtonVariant::Primary).on_click(move || {
            let current = inc_count.get();
            inc_count.set(current + 1);
            println!("Incremented to: {}", current + 1)
        }))).child(Text::new({
            if text_count.get() == 0 {
                "Counter is at zero".to_string()
            } else {
                if text_count.get() > 0 {
                    format!("Positive: {}", text_count.get())
                } else {
                    format!("Negative: {}", text_count.get())
                }
            }
        })))).to_vnode()
    };
    ReactiveApp::new("Reactive Counter".to_string(), render).run()
}

fn main() {
    start()
}

