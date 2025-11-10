use windjammer_ui::prelude::*;
use windjammer_ui::components::*;
use windjammer_ui::simple_vnode::{VNode, VAttr};



fn main() {
    let button = Button::new("Click Me".to_string()).variant(ButtonVariant::Primary);
    println!("Button created successfully!");
    let container = Container::new().max_width("1200px");
    println!("Container created successfully!");
    let text = Text::new("Hello World".to_string()).size(TextSize::Large).bold();
    println!("Text created successfully!");
    println!("All UI components work!")
}

