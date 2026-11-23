//! `wj build <file>` of a program that imports sibling modules (run from the folder that holds the files, with a bare
//! file name): every generated file goes to the `-o` directory, nothing is written next to the sources, and the output
//! is a crate that compiles, for each import form the interpreter resolves (`use lib_a::x`, `use crate::lib_a::x`,
//! `mod lib_a`).

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::path::Path;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");
const LIB_A: &str = "pub fn double(x: int) -> int { x * 2 }\n";

fn runner(import: &str) -> String {
    format!("{import}\n\nfn main() {{\n    println(\"${{double(21)}}\")\n}}\n")
}

/// Build `runner.wj` from inside `src` by its bare name into `out`; returns the file names left in `src`.
fn build_bare(src: &Path, out: &Path) -> Vec<String> {
    let o = Command::new(WJ)
        .args(["build", "runner.wj", "--target", "rust", "--no-cargo", "-o"])
        .arg(out)
        .current_dir(src)
        .env("WJ_CARGO_TARGET_ROOT", src.join("..").join("cache"))
        .output()
        .unwrap();
    assert!(o.status.success(), "{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    listing(src)
}

fn listing(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    v.sort();
    v
}

fn fixture(tag: &str, import: &str) -> (Scratch, std::path::PathBuf, std::path::PathBuf) {
    let d = Scratch::new(tag);
    let src = d.join("src_files");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("lib_a.wj"), LIB_A).unwrap();
    std::fs::write(src.join("runner.wj"), runner(import)).unwrap();
    let out = d.join("out");
    (d, src, out)
}

fn rustc_ok(args: &[&std::ffi::OsStr]) -> String {
    let o = Command::new("rustc").args(args).output().unwrap();
    let t = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(o.status.success(), "rustc failed:\n{t}");
    t
}

/// The output compiles: a library crate plus the runner as its bin when `lib.rs` is there, else the entry alone.
fn assert_output_compiles(d: &Path, out: &Path) {
    use std::ffi::OsStr;
    let edition = ["--edition", "2021"];
    let mut args: Vec<&OsStr> = edition.iter().map(OsStr::new).collect();
    let rmeta = d.join("x.rmeta");
    let mut extern_arg = String::new();
    if out.join("lib.rs").is_file() {
        let cargo = std::fs::read_to_string(out.join("Cargo.toml")).unwrap();
        let lib_name = cargo.split("[lib]\nname = \"").nth(1).and_then(|r| r.split('"').next()).unwrap().to_string();
        let runner_rs = std::fs::read_to_string(out.join("runner.rs")).unwrap();
        assert!(runner_rs.contains(&format!("{lib_name}::")), "the runner must import the lib `{lib_name}` by name:\n{runner_rs}");
        let lib = d.join("libbuilt");
        std::fs::create_dir_all(&lib).unwrap();
        let mut l: Vec<&OsStr> = args.clone();
        l.extend(["--crate-type", "lib", "--crate-name", lib_name.as_str(), "--out-dir"].map(OsStr::new));
        l.push(lib.as_os_str());
        let libs = out.join("lib.rs");
        l.push(libs.as_os_str());
        rustc_ok(&l);
        extern_arg = format!("{lib_name}={}", lib.join(format!("lib{lib_name}.rlib")).display());
        args.extend(["--extern", extern_arg.as_str()].map(OsStr::new));
    }
    args.extend(["--crate-type", "bin", "--emit", "metadata", "-o"].map(OsStr::new));
    args.push(rmeta.as_os_str());
    let runner = out.join("runner.rs");
    args.push(runner.as_os_str());
    rustc_ok(&args);
}

#[test]
fn bare_file_build_writes_everything_to_the_output_dir_and_nothing_beside_the_sources() {
    let (d, src, out) = fixture("file-o", "use lib_a::double");
    let before = listing(&src);
    let after = build_bare(&src, &out);
    assert_eq!(before, after, "the source directory must be untouched");
    for f in ["runner.rs", "lib_a.rs", "metadata.json"] {
        assert!(out.join(f).is_file(), "{f} is not in the -o directory: {:?}", listing(&out));
    }
    assert!(out.join("lib.rs").is_file(), "the imported module is part of a library crate: {:?}", listing(&out));
    assert_output_compiles(d.path(), &out);
}

#[test]
fn every_import_form_generates_the_sibling_module_and_the_output_compiles() {
    for (tag, import) in [("form-use", "use lib_a::double"), ("form-crate", "use crate::lib_a::double"), ("form-mod", "mod lib_a\nuse lib_a::double")] {
        let (d, src, out) = fixture(tag, import);
        let before = listing(&src);
        let after = build_bare(&src, &out);
        assert_eq!(before, after, "{import}: the source directory must be untouched");
        assert!(out.join("lib_a.rs").is_file(), "{import}: lib_a.rs missing from {:?}", listing(&out));
        assert_output_compiles(d.path(), &out);
    }
}
