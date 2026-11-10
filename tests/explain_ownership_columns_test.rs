//! `wj explain-ownership` reports the source column of a field read inside `${..}`, not the internal key the
//! interpolation machinery gives it (`@8:17640014`).

use std::process::Command;

#[test]
fn columns_inside_interpolations_are_source_columns() {
    let d = tempfile::TempDir::new().unwrap();
    let f = d.path().join("p.wj");
    std::fs::write(
        &f,
        "struct Playlist {\n    name: string,\n    songs: Vec<string>,\n}\n\nfn describe(list: Playlist) -> string {\n    \"${list.name}: ${list.songs.len()}\"\n}\n\nfn main() {\n    let list = Playlist { name: \"a\", songs: [] }\n    println(describe(list))\n}\n",
    )
    .unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_wj")).arg("explain-ownership").arg(&f).output().unwrap();
    let out = String::from_utf8_lossy(&o.stdout);
    for tok in out.split_whitespace().filter(|t| t.starts_with('@')) {
        let col = tok.rsplit(':').next().unwrap_or("0").parse::<usize>().unwrap_or(0);
        assert!(col < 1000, "{tok} is not a source column:\n{out}");
    }
    assert!(out.contains('@'), "{out}");
}
