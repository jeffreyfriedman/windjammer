use windjammer_runtime::ui;


fn main() {
    let app = ui::div().attr("class", "app").child(ui::h1("Hello, Windjammer UI!")).child(ui::p("This is a working Virtual DOM example")).into_vnode();
    let html = ui::render_to_string(&app);
    println!("{}", html)
}

