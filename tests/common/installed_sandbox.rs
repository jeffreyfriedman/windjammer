//! Installed-binary sandbox shared by the smoke test and the promise acceptance runner: the binary
//! under test is copied alone into a temp prefix and run with a cleared environment (empty PATH,
//! isolated HOME), so nothing but the binary exists.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

pub struct Sandbox {
    _root: TempDir,
    pub bin: PathBuf,
    pub home: PathBuf,
    pub empty_path: PathBuf,
    /// Working directories, `(label, path)`; the first is the baseline.
    pub cwds: Vec<(&'static str, PathBuf)>,
}

pub fn real_wj() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

pub fn write(p: &Path, s: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, s).unwrap();
}

impl Sandbox {
    pub fn new(bin_src: &Path) -> Sandbox {
        let root = TempDir::new().unwrap();
        let r = root.path().canonicalize().unwrap();
        let prefix = r.join("prefix/bin");
        fs::create_dir_all(&prefix).unwrap();
        let bin = prefix.join("wj");
        fs::copy(bin_src, &bin).unwrap();
        let home = r.join("home");
        fs::create_dir_all(&home).unwrap();
        let empty_path = r.join("emptypath");
        fs::create_dir_all(&empty_path).unwrap();

        let plain = r.join("cwd-plain");
        let homelike = r.join("home/work/Documents");
        let proj = r.join("cwd-project");
        let stray = r.join("cwd-stray");
        for d in [&plain, &homelike, &proj, &stray] {
            fs::create_dir_all(d).unwrap();
        }
        // A real project with wj.toml.
        write(
            &proj.join("wj.toml"),
            "[package]\nname = \"other\"\nversion = \"0.1.0\"\n",
        );
        write(
            &proj.join("src/main.wj"),
            "fn main() {\n    println(\"other\")\n}\n",
        );
        // Looks like a compiler checkout, but every asset is a decoy.
        write(&stray.join("Cargo.toml"), "[package]\nname = \"x\"\nversion = \"0.1.0\"\n[lib]\nname = \"windjammer\"\npath = \"src/lib.rs\"\n");
        write(&stray.join("src/lib.rs"), "pub fn decoy() {}\n");
        write(&stray.join("std/decoy.wj"), "pub fn decoy() {}\n");
        write(&stray.join("stdlib_meta/string.wj.meta"), "garbage");
        write(&stray.join("templates/cli/main.wj"), "DECOY");
        write(&stray.join("templates/cli/wj.toml"), "DECOY");
        write(
            &stray.join("crates/windjammer-runtime/Cargo.toml"),
            "[package]\nname = \"decoy\"\n",
        );
        write(&stray.join("agent_index/docs_index.json"), "[]");

        let checkout = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        Sandbox {
            bin,
            home,
            empty_path,
            cwds: vec![
                ("plain-tmp", plain),
                ("home-like", homelike),
                ("compiler-checkout", checkout),
                ("wj-project", proj),
                ("stray-src-lib", stray),
            ],
            _root: root,
        }
    }

    pub fn command(&self, cwd: &Path) -> Command {
        let mut c = Command::new(&self.bin);
        c.current_dir(cwd)
            .env_clear()
            .env("PATH", &self.empty_path)
            .env("HOME", &self.home)
            .env("NO_COLOR", "1");
        // Coverage runs (cargo llvm-cov) need the profile path in the child to count its lines.
        for k in ["LLVM_PROFILE_FILE", "CARGO_LLVM_COV", "CARGO_LLVM_COV_TARGET_DIR"] {
            if let Ok(v) = std::env::var(k) {
                c.env(k, v);
            }
        }
        c
    }
}
