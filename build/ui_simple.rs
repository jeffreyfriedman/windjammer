use windjammer_runtime::ui;


fn main() {
    let app = ui::div().attr("class", "container").child(ui::h1("Hello, Windjammer UI!")).child(ui::p("This is a working Virtual DOM example")).child(ui::button("Click Me")).into_vnode();
    let html = ui::render_to_string(&app);
    println!("{}", html)
}

