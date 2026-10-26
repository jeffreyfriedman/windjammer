#[derive(Debug, Clone, PartialEq, Eq, Default)]

struct Repository {
    files: Vec<String>,
}

impl Repository {
#[inline]
fn get_files(&self) -> Vec<String> {
        self.files.clone()
}
#[inline]
fn get_file_at(&self, idx: i64) -> String {
        self.files[idx as usize].clone()
}
}

#[inline]
fn test_field_then_method() {
    let repo = Repository { files: vec!["a.txt".to_string(), "b.txt".to_string()] };
    let count = count_items(&repo.get_files());
    assert!(repo.get_files().len() == 2_usize, "files should still have 2 elements");
    assert!(count == 2_i64, "count should be 2");
    println!("✓ Field then method auto-clone works");
}

#[inline]
fn test_method_then_index() {
    let repo = Repository { files: vec!["first.txt".to_string(), "second.txt".to_string(), "third.txt".to_string()] };
    let file = process_file(&repo.get_files()[0]);
    assert!(repo.get_files()[0] == "first.txt", "first file should still be first.txt");
    assert!(file == "FIRST.TXT", "processed file should be FIRST.TXT");
    println!("✓ Method then index auto-clone works");
}

#[inline]
fn test_all_combined() {
    let repo = Repository { files: vec!["x.txt".to_string(), "y.txt".to_string()] };
    let files = repo.get_files();
    let first = process_file(&files[0]);
    assert!(files[0] == "x.txt", "first file should still be x.txt");
    assert!(first == "X.TXT", "processed should be X.TXT");
    assert!(repo.get_files().len() == 2_usize, "repo files should still have 2 elements");
    println!("✓ All combined patterns auto-clone works");
}

#[inline]
fn count_items(items: &Vec<String>) -> i64 {
    items.len() as i64
}

#[inline]
fn process_file(file: &str) -> String {
    file.to_uppercase()
}

fn main() {
    test_field_then_method();
    test_method_then_index();
    test_all_combined();
    println!("✓ All combined pattern auto-clone tests passed!");
}

#[inline]
fn assert(condition: bool, message: &str) {
    if !condition {
        let _ = panic!("{}", message);
    }
}

