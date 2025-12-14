#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct User {
    pub name: String,
}

impl User {
#[inline]
pub fn new(name: String) -> User {
        User { name }
}
}

#[inline]
pub fn run() -> User {
    return User::new("Alice".to_string());
}

