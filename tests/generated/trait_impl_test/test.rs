pub trait Renderable {
    fn render(self) -> String;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
struct Text {
    content: String,
}

impl Renderable for Text {
#[inline]
fn render(self) -> String {
        self.content
}
}

