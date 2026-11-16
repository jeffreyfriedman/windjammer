//! A per-run working directory for interpreted programs.
//!
//! The working directory of a process is shared by every thread in it, so an interpreted program that called
//! `std::env::set_current_dir` on the host would pull the directory out from under everything else that runs in
//! the process (the test suite runs many interpreted programs beside compile jobs). `fs::set_current_dir` therefore
//! only records a directory on the `Interpreter`; relative paths of file and process builtins are resolved against
//! it, `current_dir` reports it, and a spawned process starts in it. A program that never changes directory is
//! untouched: nothing is rewritten until the first `set_current_dir`.

use super::{Interpreter, Value};
use std::path::{Path, PathBuf};

/// Builtins that take file-system paths, with the argument positions that are paths.
const PATH_ARGS: &[(&str, &[usize])] = &[
    ("fs::append", &[0]),
    ("fs::canonicalize", &[0]),
    ("fs::copy", &[0, 1]),
    ("fs::create_dir", &[0]),
    ("fs::create_dir_all", &[0]),
    ("fs::exists", &[0]),
    ("fs::is_dir", &[0]),
    ("fs::is_file", &[0]),
    ("fs::metadata", &[0]),
    ("fs::read", &[0]),
    ("fs::read_bytes", &[0]),
    ("fs::read_dir", &[0]),
    ("fs::read_to_string", &[0]),
    ("fs::remove_dir", &[0]),
    ("fs::remove_dir_all", &[0]),
    ("fs::remove_file", &[0]),
    ("fs::rename", &[0, 1]),
    ("fs::write", &[0]),
    ("fs::write_bytes", &[0]),
    ("fs::write_string", &[0]),
    ("path::canonicalize", &[0]),
];

impl Interpreter {
    /// The program's working directory: the one it set, else the process's.
    pub(crate) fn current_dir_io(&self) -> std::io::Result<PathBuf> {
        match &self.virtual_cwd {
            Some(d) => Ok(d.clone()),
            None => std::env::current_dir(),
        }
    }

    /// `fs::set_current_dir`: validates like the OS would, then records the directory (the process keeps its own).
    pub(crate) fn change_dir(&mut self, to: &str) -> std::io::Result<()> {
        let target = self.resolve_in_cwd(Path::new(to));
        let canonical = std::fs::canonicalize(&target)?;
        if !canonical.is_dir() {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Not a directory (os error 20)"));
        }
        self.virtual_cwd = Some(canonical);
        Ok(())
    }

    fn resolve_in_cwd(&self, p: &Path) -> PathBuf {
        match (&self.virtual_cwd, p.is_absolute()) {
            (Some(d), false) => d.join(p),
            _ => p.to_path_buf(),
        }
    }

    /// A process the program spawns starts in the program's working directory.
    pub(crate) fn start_in_cwd(&self, cmd: &mut std::process::Command) {
        if let Some(d) = &self.virtual_cwd {
            cmd.current_dir(d);
        }
    }

    /// The arguments of a file-system builtin with relative paths made absolute against the program's directory
    /// (unchanged until the program has set one).
    pub(crate) fn in_program_cwd(&self, builtin: &str, mut args: Vec<Value>) -> Vec<Value> {
        if self.virtual_cwd.is_none() {
            return args;
        }
        let Some((_, positions)) = PATH_ARGS.iter().find(|(n, _)| *n == builtin) else { return args };
        for &i in *positions {
            if let Some(Value::Str(s)) = args.get(i) {
                let p = Path::new(s.as_str());
                if !p.is_absolute() {
                    let abs = self.resolve_in_cwd(p).to_string_lossy().to_string();
                    args[i] = Value::str(abs);
                }
            }
        }
        args
    }
}
