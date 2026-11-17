//! Where the file being generated sits in the generated module tree, from declarations only: the build's output root
//! and source root (set by the driver), the module tree of the sources, and a host crate's `mod` items. No decision
//! depends on what a directory is called or on where the checkout lives.

use std::path::{Path, PathBuf};

use crate::codegen::rust::CodeGenerator;

impl CodeGenerator<'_> {
    /// Directory modules between the generated tree's root and the file being generated
    /// (`<out>/core/commands/command.rs` -> `["core", "commands"]`, `<out>/core/mod.rs` -> `["core"]`). `None` when
    /// the driver did not name the output root.
    pub(crate) fn output_module_dirs(&self) -> Option<Vec<String>> {
        let root = self.output_root.as_ref()?;
        let rel = self.current_output_file.parent()?.strip_prefix(root).ok()?;
        Some(rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect())
    }

    /// The module a host crate mounts the generated tree under: the crate root beside the output root (`lib.rs` or
    /// `main.rs` in its parent directory) declares `mod <output root>` (`pub mod generated;`). `None` when the
    /// generated tree is a crate of its own.
    pub(crate) fn host_mount_module(&self) -> Option<String> {
        let root = self.output_root.as_ref()?;
        let name = root.file_name()?.to_str()?;
        let parent = root.parent()?;
        let declares = |file: &Path| {
            std::fs::read_to_string(file).is_ok_and(|text| {
                text.lines().any(|line| {
                    let line = line.trim();
                    let item = line.strip_prefix("pub ").unwrap_or(line).trim_start();
                    item.strip_prefix("mod ").is_some_and(|rest| rest.trim_end_matches(';').trim() == name)
                })
            })
        };
        ["lib.rs", "main.rs"].iter().any(|f| declares(&parent.join(f))).then(|| name.to_string())
    }

    /// The source directory of the module `dirs` (relative to the source root): where its child modules are declared
    /// (`<src>/a/b`). A file built alone (no source root named) is its own crate root: its directory.
    fn source_dir_of(&self, dirs: &[String]) -> Option<PathBuf> {
        let mut dir = self.module_source_root.clone().or_else(|| self.current_wj_file.parent().map(PathBuf::from))?;
        for d in dirs {
            dir.push(d);
        }
        Some(dir)
    }

    /// True when the source directory of the module `dirs` declares a child module `name`: `name.wj`, a hand-written
    /// `name.rs`, or a directory module `name/`.
    pub(crate) fn source_module_declares_child(&self, dirs: &[String], name: &str) -> bool {
        let Some(dir) = self.source_dir_of(dirs) else { return false };
        dir.join(format!("{name}.wj")).is_file() || dir.join(format!("{name}.rs")).is_file() || dir.join(name).is_dir()
    }

    /// True when the source directory of the module `dirs` holds the directory module `name/`.
    pub(crate) fn source_module_has_dir_child(&self, dirs: &[String], name: &str) -> bool {
        self.source_dir_of(dirs).is_some_and(|dir| dir.join(name).is_dir())
    }
}
