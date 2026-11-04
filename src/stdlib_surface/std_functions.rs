//! Which functions each std module provides (`strings::trim`, `fs::read_to_string`, ...), so `wj check` can reject
//! a call to one that does not exist (`strings.index_of`) instead of leaving it to the interpreter or rustc.
//! A function counts when any engine provides it (see the table's header); the table is regenerated and
//! checked by `tests/std_functions_surface_test.rs`.

#[path = "std_functions_table.rs"]
mod table;

pub use table::STD_FUNCTIONS;

/// The functions of std module `module`, or `None` when the table does not cover the module (then nothing is
/// known to be missing from it).
pub fn module(module: &str) -> Option<&'static [&'static str]> {
    STD_FUNCTIONS.iter().find(|(m, _)| *m == module).map(|(_, fs)| *fs)
}

/// `Some(false)` only when `module` is covered and has no function `name`.
pub fn exists(module_name: &str, name: &str) -> Option<bool> {
    module(module_name).map(|fs| fs.contains(&name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookups() {
        assert_eq!(exists("strings", "trim"), Some(true));
        assert_eq!(exists("strings", "index_of"), Some(false));
        assert_eq!(exists("not_a_std_module", "x"), None);
        for (m, fs) in STD_FUNCTIONS {
            assert!(!fs.is_empty(), "{m}: an empty module is left out of the table");
            assert!(fs.windows(2).all(|w| w[0] < w[1]), "{m}: sorted and unique");
        }
    }
}
