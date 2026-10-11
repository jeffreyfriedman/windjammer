use std::fs;
use std::path::Path;

fn walk(dir: &Path, depth: usize, dirs: &mut u32, files: &mut u32) {
    let mut entries: Vec<(String, std::path::PathBuf, bool, u64)> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| {
            let md = e.metadata().unwrap();
            (e.file_name().to_string_lossy().into_owned(), e.path(), md.is_dir(), md.len())
        })
        .collect();
    entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    for (name, path, is_dir, size) in entries {
        let indent = "  ".repeat(depth);
        if is_dir {
            println!("{}{}/", indent, name);
            *dirs += 1;
            walk(&path, depth + 1, dirs, files);
        } else {
            println!("{}{} ({} bytes)", indent, name, size);
            *files += 1;
        }
    }
}

fn main() {
    let (mut d, mut f) = (0, 0);
    walk(Path::new("."), 0, &mut d, &mut f);
    println!("{} directories, {} files", d, f);
}
