#[component]
struct HomePage {
    visits: i64,
}

impl HomePage {
#[inline]
fn render(&self) -> VNode {
        VElement::new("div").attr("class", "page home").child(VNode::Element(VElement::new("h1").child(VNode::Text(VText::new("🏠 Home Page"))))).child(VNode::Element(VElement::new("p").child(VNode::Text(VText::new("Welcome to Windjammer UI!"))))).child(VNode::Element(VElement::new("p").child(VNode::Text(VText::new(format!("Page visits: {}", self.visits)))))).child(render_nav()).into()
}
}

#[component]
struct AboutPage {
    version: String,
}

impl AboutPage {
#[inline]
fn render(&self) -> VNode {
        VElement::new("div").attr("class", "page home").child(VNode::Element(VElement::new("h1").child(VNode::Text(VText::new("🏠 Home Page"))))).child(VNode::Element(VElement::new("p").child(VNode::Text(VText::new("Welcome to Windjammer UI!"))))).child(VNode::Element(VElement::new("p").child(VNode::Text(VText::new(format!("Page visits: {}", visits)))))).child(render_nav()).into()
}
}

#[component]
struct UserPage {
    user_id: String,
    username: String,
}

impl UserPage {
#[inline]
fn render(&self) -> VNode {
        VElement::new("div").attr("class", "page home").child(VNode::Element(VElement::new("h1").child(VNode::Text(VText::new("🏠 Home Page"))))).child(VNode::Element(VElement::new("p").child(VNode::Text(VText::new("Welcome to Windjammer UI!"))))).child(VNode::Element(VElement::new("p").child(VNode::Text(VText::new(format!("Page visits: {}", visits)))))).child(render_nav()).into()
}
}

#[component]
struct NotFoundPage {
    path: String,
}

impl NotFoundPage {
#[inline]
fn render(&self) -> VNode {
        VElement::new("div").attr("class", "page home").child(VNode::Element(VElement::new("h1").child(VNode::Text(VText::new("🏠 Home Page"))))).child(VNode::Element(VElement::new("p").child(VNode::Text(VText::new("Welcome to Windjammer UI!"))))).child(VNode::Element(VElement::new("p").child(VNode::Text(VText::new(format!("Page visits: {}", visits)))))).child(render_nav()).into()
}
}

#[inline]
fn render_nav() -> VNode {
    VElement::new("nav").child(VNode::Element(VElement::new("a").attr("href", "/").child(VNode::Text(VText::new("Home"))))).child(VNode::Text(VText::new(" | "))).child(VNode::Element(VElement::new("a").attr("href", "/about").child(VNode::Text(VText::new("About"))))).child(VNode::Text(VText::new(" | "))).child(VNode::Element(VElement::new("a").attr("href", "/users/123").child(VNode::Text(VText::new("User 123"))))).child(VNode::Text(VText::new(" | "))).child(VNode::Element(VElement::new("a").attr("href", "/search?q=windjammer").child(VNode::Text(VText::new("Search"))))).into()
}

fn main() {
    println!("=== Multi-Page Routing Example ===
");
    let router = Router::new();
    router.add_route(Route::new("/".to_string(), "HomePage".to_string()));
    router.add_route(Route::new("/about".to_string(), "AboutPage".to_string()));
    router.add_route(Route::new("/users/:id".to_string(), "UserPage".to_string()));
    println!("📋 Registered routes:");
    println!("  / -> HomePage");
    println!("  /about -> AboutPage");
    println!("  /users/:id -> UserPage
");
    println!("1️⃣  Navigating to /");
    router.navigate("/").unwrap();
    let home = HomePage { visits: 42 };
    println!("   Current page: {}", router.current().unwrap().handler);
    println!("   Rendered: {home.render():?}
");
    println!("2️⃣  Navigating to /about");
    router.navigate("/about").unwrap();
    let about = AboutPage { version: "0.34.0".to_string() };
    println!("   Current page: {}", router.current().unwrap().handler);
    println!("   Rendered: {about.render():?}
");
    println!("3️⃣  Navigating to /users/123");
    router.navigate("/users/123").unwrap();
    println!("   Current page: {}", router.current().unwrap().handler);
    println!("   Route param 'id': {}", router.param("id").unwrap());
    let user = UserPage { user_id: router.param("id").unwrap(), username: "Alice".to_string() };
    println!("   Rendered: {user.render():?}
");
    println!("4️⃣  Navigating to /search?q=windjammer&page=2");
    router.navigate("/search?q=windjammer&page=2").unwrap_or_else(|_| {
        println!("   Route not found (expected)");
    });
    match router.current() {
        Some(current) => {
            match router.query("q") {
                Some(q) => {
                    println!(format!("   Query param 'q': {}", q));
                },
            }
            match router.query("page") {
                Some(page) => {
                    println!(format!("   Query param 'page': {}", page));
                },
            }
        },
    }
    println!("
5️⃣  Going back");
    router.back().unwrap();
    println!("   Current page: {}", router.current().unwrap().handler);
    println!("
📁 File-Based Routing:");
    let mut file_router = FileBasedRouter::new("pages");
    println!("   Base directory: pages/");
    println!("   Auto-discovered routes:");
    println!("     pages/index.wj -> /");
    println!("     pages/about.wj -> /about");
    println!("     pages/users/[id].wj -> /users/:id");
    println!("     pages/blog/[...slug].wj -> /blog/*slug");
    println!("
🎯 Key Features Demonstrated:");
    println!("  ✅ Static routes (/about)");
    println!("  ✅ Dynamic routes (/users/:id)");
    println!("  ✅ Query parameters (?q=value)");
    println!("  ✅ Navigation history (back/forward)");
    println!("  ✅ File-based routing");
    println!("  ✅ Route parameters extraction");
    println!("  ✅ Not found handling")
}

