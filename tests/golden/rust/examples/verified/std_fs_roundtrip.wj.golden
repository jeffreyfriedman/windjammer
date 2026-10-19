use windjammer_runtime::fs;
use windjammer_runtime::fs::DirEntry;
#[inline]
fn roundtrip(path: String) -> Result<String, String> {
    fs::write(&path, "line1\nline2\n")?;
    let text = fs::read_to_string(&path)?;
    fs::remove_file(&path)?;
    Ok(text)
}

fn main() {
    let path: String = String::from("/tmp/wj_verified_fs_roundtrip.txt");
    match roundtrip(path.to_string()) {
        Ok(text) => {
            windjammer_runtime::io::println(&format!("wrote {} lines", text.lines().count()));
            for l in text.lines() {
                windjammer_runtime::io::println(&format!("{}", l));
            }
        },
        Err(e) => windjammer_runtime::io::println(&format!("io error: {}", e)),
    }
    windjammer_runtime::io::println(&format!("exists: {}", fs::exists(&path)));
}

