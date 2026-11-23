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

    /// Names whose declarations agree, with their types (what codegen imports across modules).
    pub fn agreed(&self) -> HashMap<String, Type> {
        self.global.iter().filter_map(|(n, t)| t.clone().map(|t| (n.clone(), t))).collect()
    }
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
