//! The `const` / `static` declarations numeric inference knows, looked up by declaration: a name resolves to the
//! declaration of the file that uses it, and otherwise only when every declaration of that name agrees. Two modules
//! that declare `TAU` with different types never answer each other's lookups (no bare-name guess).

use crate::parser::Type;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ConstTable {
    by_file: HashMap<(String, String), Type>,
    /// Name -> its type when every declaration agrees; `None` when two declarations disagree.
    global: HashMap<String, Option<Type>>,
}

impl ConstTable {
    pub fn insert(&mut self, file: &str, name: String, ty: Type) {
        self.by_file.insert((file.to_string(), name.clone()), ty.clone());
        match self.global.get(&name) {
            None => {
                self.global.insert(name, Some(ty));
            }
            Some(Some(prev)) if *prev == ty => {}
            Some(_) => {
                self.global.insert(name, None);
            }
        }
    }

    /// The type of `name` as seen from `file`.
    pub fn get(&self, file: &str, name: &str) -> Option<&Type> {
        if let Some((module, item)) = name.rsplit_once("::") {
            return self.get_in_module(module, item);
        }
        self.by_file
            .get(&(file.to_string(), name.to_string()))
            .or_else(|| self.global.get(name).and_then(Option::as_ref))
    }

    /// `module::NAME` (`narrow::TAU`, `crate::narrow::TAU`): the constant the module's own file declares. The module
    /// is the file `<module>.wj` or `<module>/mod.wj` (the file `mod <module>` declares).
    fn get_in_module(&self, module: &str, item: &str) -> Option<&Type> {
        let seg = module.rsplit("::").next().unwrap_or(module);
        let mut found = self.by_file.iter().filter(|((file, name), _)| name == item && file_is_module(file, seg));
        let first = found.next()?;
        if found.next().is_some() {
            return None;
        }
        Some(first.1)
    }

    /// True when some module declares `name` (also when the declarations disagree).
    pub fn is_declared(&self, name: &str) -> bool {
        if let Some((module, item)) = name.rsplit_once("::") {
            return self.get_in_module(module, item).is_some();
        }
        self.global.contains_key(name)
    }

    /// Register the constants of another crate that a `use` names: `imported` is the imported name (`GAIN` for
    /// `use dep::GAIN`, `*` for a glob). `consts` is that crate's declared constants by name (its `metadata.json`).
    pub fn load_imported_crate_consts(&mut self, crate_key: &str, consts: &HashMap<String, Type>, imported: &str) {
        for (name, ty) in consts {
            if name == imported || imported == "*" {
                self.insert(crate_key, name.clone(), ty.clone());
            }
        }
    }

    pub fn extend(&mut self, other: ConstTable) {
        for ((file, name), ty) in other.by_file {
            self.insert(&file, name, ty);
        }
        for (name, ty) in other.global {
            if ty.is_none() {
                self.global.insert(name, None);
            }
        }
    }

    /// The table of every top-level `const` / `static` of a crate, read from the parsed programs of all its files
    /// (the crate-wide pre-pass: no cached metadata and no file order involved).
    pub fn from_programs(programs: &[crate::parser::Program<'_>]) -> Self {
        use crate::parser::ast::core::Item;
        let mut table = ConstTable::default();
        for program in programs {
            let Some(file) = program.items.first().and_then(|i| i.location()).map(|l| l.file.to_string_lossy().to_string())
            else {
                continue;
            };
            for item in &program.items {
                if let Item::Const { name, type_, .. } | Item::Static { name, type_, .. } = item {
                    table.insert(&file, name.clone(), type_.clone());
                }
            }
        }
        table
    }

    /// `module::NAME` -> type for every constant whose module (the declaring file's `<module>.wj` or
    /// `<module>/mod.wj`) is the only file of that name declaring it: how a qualified use (`small::LIMIT`, and the
    /// `use small::LIMIT` import) finds the declaration, whatever other modules call their own `LIMIT`.
    pub fn qualified(&self) -> HashMap<String, Type> {
        let mut out: HashMap<String, Option<Type>> = HashMap::new();
        for ((file, name), ty) in &self.by_file {
            let Some(module) = module_of_file(file) else { continue };
            let key = format!("{module}::{name}");
            match out.get(&key) {
                None => {
                    out.insert(key, Some(ty.clone()));
                }
                Some(Some(prev)) if prev == ty => {}
                Some(_) => {
                    out.insert(key, None);
                }
            }
        }
        out.into_iter().filter_map(|(k, t)| t.map(|t| (k, t))).collect()
    }

    /// Names whose declarations agree, with their types (what codegen imports across modules).
    pub fn agreed(&self) -> HashMap<String, Type> {
        self.global.iter().filter_map(|(n, t)| t.clone().map(|t| (n.clone(), t))).collect()
    }
}

fn module_of_file(file: &str) -> Option<String> {
    let path = std::path::Path::new(file);
    let stem = path.file_stem().and_then(|s| s.to_str())?;
    if stem == "mod" {
        return path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()).map(str::to_string);
    }
    Some(stem.to_string())
}

fn file_is_module(file: &str, module: &str) -> bool {
    let path = std::path::Path::new(file);
    let stem = path.file_stem().and_then(|s| s.to_str());
    match stem {
        Some("mod") => path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some(module),
        Some(s) => s == module,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The crate-wide pre-pass reads every file's constants from the parsed programs: two modules declaring
    /// `LIMIT` at different widths agree on no bare name, and each is reachable as `module::LIMIT`.
    #[test]
    fn crate_wide_table_resolves_same_named_consts_by_module() {
        let parse = |file: &str, src: &str| {
            let tokens = crate::lexer::Lexer::new(src).tokenize_with_locations();
            let parser = Box::leak(Box::new(crate::parser::Parser::new_with_source(tokens, file.to_string(), src.to_string())));
            parser.parse().expect("parse")
        };
        let programs = vec![
            parse("/p/small.wj", "pub const LIMIT: u8 = 200\n"),
            parse("/p/big.wj", "pub const LIMIT: u64 = 5000000000\n"),
            parse("/p/main.wj", "fn main() {\n}\n"),
        ];
        let table = ConstTable::from_programs(&programs);
        assert!(table.agreed().is_empty(), "disagreeing bare names are not agreed");
        let q = table.qualified();
        assert_eq!(q.get("small::LIMIT"), Some(&Type::Custom("u8".to_string())));
        assert_eq!(q.get("big::LIMIT"), Some(&Type::Custom("u64".to_string())));
    }
}
