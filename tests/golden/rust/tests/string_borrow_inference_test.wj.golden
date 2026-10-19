use windjammer_runtime::test::*;
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct Logger {
    pub prefix: String,
}

impl Logger {
#[inline]
pub fn new(name: String) -> Logger {
        Logger { prefix: name }
}
}

#[inline]
pub fn store_path(paths: &mut Vec<String>, path: String) {
    paths.push(path);
}

#[test]

pub fn test_string_storage() {
    let logger = Logger::new("App".to_string());
    let mut paths: Vec<String> = Vec::new();
    store_path(&mut paths, "file.txt".to_string());
    println!("Logger prefix: {}", logger.prefix);
}

fn main() {
    test_string_storage();
}

