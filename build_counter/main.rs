use windjammer_ui::prelude::*;
use windjammer_ui::components::*;
use windjammer_ui::simple_vnode::{VNode, VAttr};
use wasm_bindgen::prelude::*;



#[inline]
#[wasm_bindgen]
pub fn start() {
    println!("🔢 Starting Interactive Counter");
    let count = Signal::new(0);
    let ui = Container::new().max_width("600px").child(Panel::new("Interactive Counter".to_string()).child(Flex::new().direction(FlexDirection::Column).gap("20px").child(Text::new(format!("Count: {}", count.get())).size(TextSize::XLarge)).child(Flex::new().direction(FlexDirection::Row).gap("10px").child(Button::new("- Decrement".to_string()).variant(ButtonVariant::Secondary).on_click(move || {
        let current = count.get();
        count.set(current - 1);
        println!("Decremented to: {}", current - 1)
    })).child(Button::new("Reset".to_string()).variant(ButtonVariant::Danger).on_click(move || {
        count.set(0);
        println!("Reset to 0")
    })).child(Button::new("+ Increment".to_string()).variant(ButtonVariant::Primary).on_click(move || {
        let current = count.get();
        count.set(current + 1);
        println!("Incremented to: {}", current + 1)
    }))).child(Text::new({
        if count.get() == 0 {
            "Counter is at zero".to_string()
        } else {
            if count.get() > 0 {
                format!("Positive: {}", count.get())
            } else {
                format!("Negative: {}", count.get())
            }
        }
    }))));
    App::new("Interactive Counter".to_string(), ui.to_vnode()).run()
}

fn main() {
    start()
}

