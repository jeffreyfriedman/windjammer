pub mod fs {
pub fn read_to_string(path: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}

pub fn read(path: &str) -> Result<Vec<u8>, std::io::Error> {
    std::fs::read(path)
}

pub fn write(path: &str, contents: &str) -> Result<(), std::io::Error> {
    std::fs::write(path, contents)
}

pub fn write_bytes(path: &str, contents: &[u8]) -> Result<(), std::io::Error> {
    std::fs::write(path, contents)
}

pub fn exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

pub fn is_file(path: &str) -> bool {
    std::path::Path::new(path).is_file()
}

pub fn is_dir(path: &str) -> bool {
    std::path::Path::new(path).is_dir()
}

pub fn create_dir_all(path: &str) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(path)
}

pub fn remove_file(path: &str) -> Result<(), std::io::Error> {
    std::fs::remove_file(path)
}

pub fn remove_dir(path: &str) -> Result<(), std::io::Error> {
    std::fs::remove_dir(path)
}

pub fn remove_dir_all(path: &str) -> Result<(), std::io::Error> {
    std::fs::remove_dir_all(path)
}

pub fn copy(from: &str, to: &str) -> Result<u64, std::io::Error> {
    std::fs::copy(from, to)
}

pub fn rename(from: &str, to: &str) -> Result<(), std::io::Error> {
    std::fs::rename(from, to)
}


}




fn main() {
    println!("=== File System Module Test ===
");
    println!("1. Check /tmp directory:");
    if fs::exists("/tmp") {
        println!("✓ /tmp exists")
    } else {
        println!("✗ /tmp doesn't exist")
    }
    if fs::is_dir("/tmp") {
        println!("✓ /tmp is a directory")
    } else {
        println!("✗ /tmp is not a directory")
    }
    println!("
✅ File system test complete!")
}

