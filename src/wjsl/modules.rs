//! WJSL modules (docs/design/wjsl-modules.md, stage S1): `use crate::module::item` between `.wjsl`
//! files of ONE folder. A file `brdf.wjsl` is module `brdf`; the folder of the entry file is `crate`.
//!
//! Pipeline: load graph (includes, `__` ban, top-level scan, cycle/missing checks) -> resolve imports
//! (visibility, collisions) -> rewrite each module's source to module-qualified names -> parse each module
//! separately -> merge into one `ShaderModule` for the checker -> dead-code elimination -> the one WGSL printer.
//!
//! Invariants (docs/design/wjsl-modules-soundness.md): names are a pure function of `(module stem, item)`;
//! output depends only on source text and module stems, never on paths, directory listings or hash order.

use crate::wjsl::ast::*;
use crate::wjsl::diagnostics::ShaderDiagnostics;
use crate::wjsl::includes::resolve_includes;
use anyhow::{anyhow, Result};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

/// Emitted name of item `name` declared in library module `module`: the single mangling function
/// (shared with the S2 CPU twin, which keeps the `(module, name)` key itself).
pub fn qualified_name(module: &str, name: &str) -> String {
    format!("{module}__{name}")
}

fn diag(code: &str, file: &str, line: usize, col: usize, msg: &str, fix: &str) -> anyhow::Error {
    anyhow!("{file}:{line}:{col}: error[{code}]: {msg}\n  fix: {fix}")
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ItemKind {
    Fn,
    Struct,
    Const,
}

#[derive(Clone, Debug)]
struct ItemInfo {
    name: String,
    kind: ItemKind,
    is_pub: bool,
    line: usize,
}

#[derive(Clone, Debug)]
enum UseKind {
    /// `use crate::brdf`
    Module,
    /// `use crate::brdf::item`
    Item(String, usize, usize),
    /// `use crate::brdf::*`
    Glob,
}

#[derive(Clone, Debug)]
struct UseDecl {
    module: String,
    kind: UseKind,
    line: usize,
    /// column of the module name
    col: usize,
}

struct Scan {
    /// source with `use` lines blanked and `pub ` replaced by spaces (line and column preserving)
    text: String,
    uses: Vec<UseDecl>,
    items: Vec<ItemInfo>,
    /// bindings, globals and entry points: (what, line, col)
    forbidden: Vec<(&'static str, usize, usize)>,
}

struct ModuleSrc {
    stem: String,
    file: String,
    scan: Scan,
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}
fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Net brace depth change of one line, ignoring comments. `in_block` carries `/* */` state.
fn brace_delta(line: &str, in_block: &mut bool) -> i32 {
    let ch: Vec<char> = line.chars().collect();
    let mut d = 0;
    let mut i = 0;
    while i < ch.len() {
        if *in_block {
            if ch[i] == '*' && ch.get(i + 1) == Some(&'/') {
                *in_block = false;
                i += 1;
            }
        } else if ch[i] == '/' && ch.get(i + 1) == Some(&'/') {
            break;
        } else if ch[i] == '/' && ch.get(i + 1) == Some(&'*') {
            *in_block = true;
            i += 1;
        } else if ch[i] == '{' {
            d += 1;
        } else if ch[i] == '}' {
            d -= 1;
        }
        i += 1;
    }
    d
}

/// Identifiers of `text` with 1-based line and column, skipping comments and numeric literals.
fn identifiers(text: &str) -> Vec<(String, usize, usize)> {
    let ch: Vec<char> = text.chars().collect();
    let (mut i, mut line, mut col) = (0, 1, 1);
    let mut out = Vec::new();
    while i < ch.len() {
        let c = ch[i];
        if c == '/' && ch.get(i + 1) == Some(&'/') {
            while i < ch.len() && ch[i] != '\n' {
                i += 1;
                col += 1;
            }
            continue;
        }
        if c == '/' && ch.get(i + 1) == Some(&'*') {
            i += 2;
            col += 2;
            while i < ch.len() && !(ch[i] == '*' && ch.get(i + 1) == Some(&'/')) {
                if ch[i] == '\n' {
                    line += 1;
                    col = 0;
                }
                i += 1;
                col += 1;
            }
            i += 2;
            col += 2;
            continue;
        }
        if c.is_ascii_digit() {
            while i < ch.len() && (is_ident_char(ch[i]) || ch[i] == '.') {
                i += 1;
                col += 1;
            }
            continue;
        }
        if is_ident_start(c) {
            let start = i;
            let (l0, c0) = (line, col);
            while i < ch.len() && is_ident_char(ch[i]) {
                i += 1;
                col += 1;
            }
            out.push((ch[start..i].iter().collect(), l0, c0));
            continue;
        }
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
        i += 1;
    }
    out
}

/// `__` is reserved for module-qualified emitted names (WJ0201).
fn check_reserved(text: &str, file: &str) -> Result<()> {
    for (name, line, col) in identifiers(text) {
        if name.contains("__") {
            return Err(diag(
                "WJ0201",
                file,
                line,
                col,
                &format!("`{name}` contains `__`, which is reserved for module-qualified names"),
                "rename the identifier without a double underscore",
            ));
        }
    }
    Ok(())
}

/// Scan the top level (brace depth 0) for `use crate::...` lines, `pub` items and forbidden declarations.
fn scan_top_level(source: &str, file: &str) -> Result<Scan> {
    let mut depth = 0i32;
    let mut in_block = false;
    let mut text = String::with_capacity(source.len());
    let (mut uses, mut items, mut forbidden) = (Vec::new(), Vec::new(), Vec::new());
    let src_lines: Vec<&str> = source.lines().collect();
    let mut idx = 0usize;
    while idx < src_lines.len() {
        let raw = src_lines[idx];
        let line_no = idx + 1;
        idx += 1;
        let at_top = depth == 0 && !in_block;
        let mut line = raw.to_string();
        if at_top {
            let indent = raw.len() - raw.trim_start().len();
            let trimmed = &raw[indent..];
            for (kw, advice) in [
                ("import", "WJSL has no `import`: write `use crate::module::item` (module import, same folder) or `use \"path\"` (text include)"),
                ("mod", "WJSL has no `mod`: a module is a file; write `use crate::module::item` to import from `module.wjsl` in the same folder"),
            ] {
                let is_kw = trimmed.strip_prefix(kw).is_some_and(|r| r.starts_with(|c: char| c.is_whitespace() || c == '"'));
                if is_kw {
                    return Err(diag(
                        "WJ0197",
                        file,
                        line_no,
                        indent + 1,
                        advice,
                        "replace the line with a `use` import",
                    ));
                }
            }
            if let Some(rest) = trimmed.strip_prefix("use ") {
                if !rest.trim_start().starts_with('"') {
                    // a `{a,\n b}` list continues until its closing brace
                    let mut stmt: Vec<(usize, &str)> = vec![(line_no, raw)];
                    if raw.contains('{') && !raw.contains('}') {
                        while idx < src_lines.len() {
                            let l = src_lines[idx];
                            stmt.push((idx + 1, l));
                            idx += 1;
                            if l.contains('}') {
                                break;
                            }
                        }
                    }
                    parse_use(&stmt, file, &mut uses)?;
                    let _ = rest;
                    for _ in &stmt {
                        text.push('\n');
                    }
                    continue;
                }
            }
            let (is_pub, decl, decl_col) = match trimmed.strip_prefix("pub ") {
                Some(r) => (true, r.trim_start(), raw.len() - r.trim_start().len()),
                None => (false, trimmed, indent),
            };
            let _ = decl_col;
            if is_pub {
                line = format!("{}{}", " ".repeat(raw.len() - decl.len()), decl);
            }
            for (kw, kind) in [("fn ", ItemKind::Fn), ("struct ", ItemKind::Struct), ("const ", ItemKind::Const)] {
                if let Some(r) = decl.strip_prefix(kw) {
                    let name: String = r.trim_start().chars().take_while(|c| is_ident_char(*c)).collect();
                    if !name.is_empty() {
                        items.push(ItemInfo { name, kind, is_pub, line: line_no });
                    }
                }
            }
            if is_pub && !["fn ", "struct ", "const "].iter().any(|k| decl.starts_with(k)) {
                return Err(diag(
                    "WJ0200",
                    file,
                    line_no,
                    indent + 1,
                    "`pub` applies to `fn`, `struct` and `const` only",
                    "remove `pub` or put it before a function, struct or const",
                ));
            }
            for (pre, what) in [
                ("@vertex", "entry point"),
                ("@fragment", "entry point"),
                ("@compute", "entry point"),
                ("@group", "binding"),
                ("var<", "global variable"),
            ] {
                if decl.starts_with(pre) {
                    forbidden.push((what, line_no, indent + 1));
                }
            }
        }
        text.push_str(&line);
        text.push('\n');
        depth += brace_delta(raw, &mut in_block);
    }
    Ok(Scan { text, uses, items, forbidden })
}

/// Parse one `use crate::...` statement. `stmt` holds its source lines (1-based number, text): a `{a, b}` list
/// may continue over several lines.
fn parse_use(stmt: &[(usize, &str)], file: &str, out: &mut Vec<UseDecl>) -> Result<()> {
    let first = stmt[0].1.trim_start().strip_prefix("use ").unwrap_or("");
    let mut joined = first.trim().to_string();
    for (_, l) in &stmt[1..] {
        joined.push(' ');
        joined.push_str(l.trim());
    }
    let body = joined.trim().trim_end_matches(';').trim().to_string();
    let body = body.as_str();
    // position of the next whole-word occurrence of `needle`, scanning forward from `cursor` (line index, byte)
    let locate = |needle: &str, cursor: &mut (usize, usize)| -> (usize, usize) {
        for li in cursor.0..stmt.len() {
            let text = stmt[li].1;
            let from = if li == cursor.0 { cursor.1 } else { 0 };
            let mut at = from;
            while let Some(p) = text.get(at..).and_then(|t| t.find(needle)) {
                let pos = at + p;
                let before = text[..pos].chars().next_back();
                let after = text[pos + needle.len()..].chars().next();
                if !before.is_some_and(is_ident_char) && !after.is_some_and(is_ident_char) {
                    *cursor = (li, pos + needle.len());
                    return (stmt[li].0, pos + 1);
                }
                at = pos + needle.len();
            }
        }
        (stmt[0].0, 1)
    };
    let mut cur = (0usize, 0usize);
    let (line0, col0) = locate("use", &mut cur);
    let bad = |msg: &str| {
        diag(
            "WJ0197",
            file,
            line0,
            col0,
            msg,
            "write `use crate::module`, `use crate::module::item`, `use crate::module::{a, b}` or `use crate::module::*`",
        )
    };
    let segs: Vec<&str> = body.splitn(3, "::").collect();
    if segs.len() < 2 || segs[0] != "crate" {
        return Err(bad(&format!("unsupported import `{body}`: write `use crate::module::item` (same folder) or `use \"path\"` (text include)")));
    }
    let module = segs[1].trim().to_string();
    if module.is_empty() || !module.chars().all(is_ident_char) {
        return Err(bad(&format!("`{module}` is not a module name")));
    }
    locate("crate", &mut cur);
    let (mline, mcol) = locate(&module, &mut cur);
    if segs.len() == 2 {
        out.push(UseDecl { module, kind: UseKind::Module, line: mline, col: mcol });
        return Ok(());
    }
    let tail = segs[2].trim();
    if tail == "*" {
        out.push(UseDecl { module, kind: UseKind::Glob, line: mline, col: mcol });
    } else if let Some(inner) = tail.strip_prefix('{').and_then(|t| t.strip_suffix('}')) {
        for name in inner.split(',').map(str::trim).filter(|n| !n.is_empty()) {
            if !name.chars().all(is_ident_char) {
                return Err(bad(&format!("`{name}` is not an item name")));
            }
            let (iline, icol) = locate(name, &mut cur);
            out.push(UseDecl { module: module.clone(), kind: UseKind::Item(name.to_string(), iline, icol), line: mline, col: mcol });
        }
    } else if tail.chars().all(is_ident_char) && !tail.is_empty() {
        let (iline, icol) = locate(tail, &mut cur);
        out.push(UseDecl { module, kind: UseKind::Item(tail.to_string(), iline, icol), line: mline, col: mcol });
    } else {
        return Err(bad(&format!("unsupported import `{body}` (submodules are not supported yet)")));
    }
    Ok(())
}

struct Graph {
    mods: BTreeMap<String, ModuleSrc>,
    /// depth-first post-order of module stems, imports sorted by stem; the entry key is last
    order: Vec<String>,
    entry: String,
}

fn load_module(dir: &Path, stem: &str, file: &str, source: &str, is_entry: bool) -> Result<ModuleSrc> {
    let resolved = resolve_includes(source, dir, &mut Vec::new()).map_err(|e| ShaderDiagnostics::new(source).enrich(e))?;
    check_reserved(&resolved, file)?;
    let scan = scan_top_level(&resolved, file)?;
    if !is_entry {
        if let Some((what, line, col)) = scan.forbidden.first() {
            return Err(diag(
                "WJ0200",
                file,
                *line,
                *col,
                &format!("a {what} is not allowed in library module `{stem}`: bindings, globals and entry points belong to the entry file"),
                "move it to the entry file and pass the resource as a parameter",
            ));
        }
    }
    Ok(ModuleSrc { stem: stem.to_string(), file: file.to_string(), scan })
}

fn visit(dir: &Path, g: &mut Graph, stack: &mut Vec<String>, key: &str) -> Result<()> {
    let mut deps: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for u in &g.mods[key].scan.uses {
        deps.entry(u.module.clone()).or_insert((u.line, u.col));
    }
    stack.push(key.to_string());
    for (dep, (line, col)) in deps {
        let importer_file = g.mods[key].file.clone();
        if let Some(pos) = stack.iter().position(|s| *s == dep) {
            let mut chain: Vec<String> = stack[pos..].iter().map(|s| format!("{s}.wjsl")).collect();
            chain.push(format!("{dep}.wjsl"));
            return Err(diag(
                "WJ0199",
                &importer_file,
                line,
                col,
                &format!("import cycle: {}", chain.join(" -> ")),
                "break the cycle: move the shared items into a third module that both import",
            ));
        }
        if g.mods.contains_key(&dep) {
            continue;
        }
        let file = format!("{dep}.wjsl");
        let path = dir.join(&file);
        let source = std::fs::read_to_string(&path).map_err(|_| {
            diag(
                "WJ0197",
                &importer_file,
                line,
                col,
                &format!("no module `{dep}` in `crate` (looked for `{file}` next to `{importer_file}`)"),
                &format!("create `{file}` in the same folder or fix the module name"),
            )
        })?;
        let m = load_module(dir, &dep, &file, &source, false)?;
        g.mods.insert(dep.clone(), m);
        visit(dir, g, stack, &dep)?;
    }
    stack.pop();
    g.order.push(key.to_string());
    Ok(())
}

/// Per-module name resolution: the rename map for the module's own source text.
struct Resolution {
    /// bare identifier -> emitted name
    map: HashMap<String, String>,
    /// `use crate::m` aliases (module stems)
    aliases: BTreeSet<String>,
}

fn resolve_imports(g: &Graph, key: &str) -> Result<Resolution> {
    let m = &g.mods[key];
    let is_entry = key == g.entry;
    let mut map: HashMap<String, String> = HashMap::new();
    let mut origin_of: HashMap<String, String> = HashMap::new(); // name -> importing module (collision text)
    let mut aliases = BTreeSet::new();
    let locals: HashSet<&str> = m.scan.items.iter().map(|i| i.name.as_str()).collect();
    if !is_entry {
        for it in &m.scan.items {
            map.insert(it.name.clone(), qualified_name(&m.stem, &it.name));
        }
    }
    let mut globs: Vec<&UseDecl> = Vec::new();
    for u in &m.scan.uses {
        let target = &g.mods[&u.module];
        match &u.kind {
            UseKind::Module => {
                aliases.insert(u.module.clone());
            }
            UseKind::Glob => globs.push(u),
            UseKind::Item(name, iline, col) => {
                let Some(item) = target.scan.items.iter().find(|i| i.name == *name) else {
                    return Err(diag(
                        "WJ0197",
                        &m.file,
                        *iline,
                        *col,
                        &format!("no function, struct or const `{name}` in module `{}`", u.module),
                        "check the spelling or the module's `pub` items",
                    ));
                };
                if !item.is_pub {
                    let kw = match item.kind {
                        ItemKind::Fn => "fn",
                        ItemKind::Struct => "struct",
                        ItemKind::Const => "const",
                    };
                    return Err(diag(
                        "WJ0198",
                        &m.file,
                        *iline,
                        *col,
                        &format!("`{name}` is private to `{}`", u.module),
                        &format!("declare it `pub {kw} {name}` in `{}`", target.file),
                    ));
                }
                if locals.contains(name.as_str()) || origin_of.get(name).is_some_and(|o| *o != u.module) {
                    return Err(diag(
                        "WJ0197",
                        &m.file,
                        *iline,
                        *col,
                        &format!("`{name}` is imported from `{}` but already declared or imported here", u.module),
                        "rename the local item or import only one of them",
                    ));
                }
                origin_of.insert(name.clone(), u.module.clone());
                map.insert(name.clone(), qualified_name(&u.module, name));
            }
        }
    }
    // Glob imports rank below local and explicit names; two globs bringing one name is an error.
    let mut glob_from: HashMap<String, &str> = HashMap::new();
    for u in globs {
        for it in g.mods[&u.module].scan.items.iter().filter(|i| i.is_pub) {
            if locals.contains(it.name.as_str()) || origin_of.contains_key(&it.name) {
                continue;
            }
            if let Some(prev) = glob_from.get(&it.name) {
                if *prev != u.module {
                    return Err(diag(
                        "WJ0197",
                        &m.file,
                        u.line,
                        u.col,
                        &format!("`{}` is brought in by both `{prev}` and `{}`", it.name, u.module),
                        "import it explicitly: `use crate::module::name`",
                    ));
                }
            }
            glob_from.insert(it.name.clone(), &u.module);
            map.insert(it.name.clone(), qualified_name(&u.module, &it.name));
        }
    }
    Ok(Resolution { map, aliases })
}

/// Rewrite one module's source so every top-level name is its emitted name.
fn rewrite(text: &str, g: &Graph, key: &str, res: &Resolution) -> Result<String> {
    let file = &g.mods[key].file;
    let ch: Vec<char> = text.chars().collect();
    let (mut i, mut line, mut col) = (0usize, 1usize, 1usize);
    let mut out = String::with_capacity(text.len() + 64);
    let n = ch.len();
    // Binding-aware renaming: WGSL scopes, innermost last. A name bound by a parameter, `let`/`var`/`const`
    // or `for` variable is that binding, never the module-level or imported item of the same name.
    let mut scopes: Vec<(HashSet<String>, bool)> = Vec::new(); // (names, is a `for` header scope)
    let mut in_fn_header = false;
    let mut paren = 0i32;
    let mut params: HashSet<String> = HashSet::new();
    let mut expect_decl = false;
    let mut pending: Option<String> = None;
    let mut prev_ident = String::new();
    let step = |c: char, line: &mut usize, col: &mut usize| {
        if c == '\n' {
            *line += 1;
            *col = 1;
        } else {
            *col += 1;
        }
    };
    while i < n {
        let c = ch[i];
        if c == '/' && ch.get(i + 1) == Some(&'/') {
            while i < n && ch[i] != '\n' {
                out.push(ch[i]);
                i += 1;
                col += 1;
            }
            continue;
        }
        if c == '/' && ch.get(i + 1) == Some(&'*') {
            out.push_str("/*");
            i += 2;
            col += 2;
            while i < n && !(ch[i] == '*' && ch.get(i + 1) == Some(&'/')) {
                out.push(ch[i]);
                step(ch[i], &mut line, &mut col);
                i += 1;
            }
            if i < n {
                out.push_str("*/");
                i += 2;
                col += 2;
            }
            continue;
        }
        if c.is_ascii_digit() {
            while i < n && (is_ident_char(ch[i]) || ch[i] == '.') {
                out.push(ch[i]);
                i += 1;
                col += 1;
            }
            continue;
        }
        if !is_ident_start(c) {
            match c {
                '(' if in_fn_header => paren += 1,
                ')' if in_fn_header => paren -= 1,
                '{' => {
                    if in_fn_header && paren == 0 {
                        scopes.push((std::mem::take(&mut params), false));
                        in_fn_header = false;
                    } else {
                        scopes.push((HashSet::new(), false));
                    }
                }
                '}' => {
                    scopes.pop();
                    if scopes.last().is_some_and(|s| s.1) {
                        scopes.pop(); // the `for` header scope ends with its body
                    }
                }
                ';' => {
                    if let Some(name) = pending.take() {
                        if let Some(top) = scopes.last_mut() {
                            top.0.insert(name);
                        }
                    }
                }
                _ => {}
            }
            out.push(c);
            step(c, &mut line, &mut col);
            i += 1;
            continue;
        }
        // identifier chain `a::b::c`
        let (l0, c0, start) = (line, col, i);
        let mut segs: Vec<String> = Vec::new();
        loop {
            let s = i;
            while i < n && is_ident_char(ch[i]) {
                i += 1;
            }
            segs.push(ch[s..i].iter().collect());
            if ch.get(i) == Some(&':') && ch.get(i + 1) == Some(&':') && ch.get(i + 2).is_some_and(|c| is_ident_start(*c)) {
                i += 2;
            } else {
                break;
            }
        }
        col += i - start;
        let verbatim: String = ch[start..i].iter().collect();
        let prev = ch[..start].iter().rev().find(|c| **c != ' ' && **c != '\t').copied();
        let glued_prev = if start > 0 { Some(ch[start - 1]) } else { None };
        let next = ch[i..].iter().find(|c| **c != ' ' && **c != '\t').copied();
        let next2 = {
            let mut it = ch[i..].iter().skip_while(|c| **c == ' ' || **c == '\t');
            it.next();
            it.next().copied()
        };
        // `const NAME: T = ..` at module level declares an item: it is renamed even though a `:` follows
        let item_decl = scopes.is_empty() && !in_fn_header && prev_ident == "const";
        let declared_name = next == Some(':') && next2 != Some(':') && !item_decl;
        prev_ident = segs[0].clone();
        if prev == Some('.') || glued_prev == Some('@') {
            out.push_str(&verbatim);
            continue;
        }
        if segs.len() == 1 {
            let name = segs[0].as_str();
            if name == "fn" && scopes.is_empty() {
                in_fn_header = true;
                paren = 0;
                params.clear();
            } else if name == "for" && !scopes.is_empty() {
                scopes.push((HashSet::new(), true));
            } else if !scopes.is_empty() && matches!(name, "let" | "var" | "const") {
                expect_decl = true;
            } else if expect_decl && matches!(prev, Some('<') | Some(',')) {
                // `var<private>` / `var<storage, read_write>`: the address space is not the declared name
            } else if expect_decl {
                expect_decl = false;
                pending = Some(name.to_string());
                out.push_str(&verbatim);
                continue;
            } else if in_fn_header && paren == 1 && declared_name {
                params.insert(name.to_string());
            }
            if scopes.iter().any(|(set, _)| set.contains(name)) {
                out.push_str(&verbatim);
                continue;
            }
            match res.map.get(&segs[0]) {
                Some(q) if !declared_name => out.push_str(q),
                _ => out.push_str(&verbatim),
            }
            continue;
        }
        // qualified: `alias::item` or `crate::module::item`
        let (module, item) = if segs[0] == "crate" && segs.len() == 3 {
            (segs[1].clone(), segs[2].clone())
        } else if segs.len() == 2 && res.aliases.contains(&segs[0]) {
            (segs[0].clone(), segs[1].clone())
        } else {
            out.push_str(&verbatim);
            continue;
        };
        if !g.mods[key].scan.uses.iter().any(|u| u.module == module) {
            return Err(diag(
                "WJ0197",
                file,
                l0,
                c0,
                &format!("module `{module}` is not imported"),
                &format!("add `use crate::{module}`"),
            ));
        }
        let target = &g.mods[&module];
        match target.scan.items.iter().find(|it| it.name == item) {
            None => {
                return Err(diag(
                    "WJ0197",
                    file,
                    l0,
                    c0,
                    &format!("no function, struct or const `{item}` in module `{module}`"),
                    "check the spelling or the module's `pub` items",
                ))
            }
            Some(it) if !it.is_pub => {
                return Err(diag(
                    "WJ0198",
                    file,
                    l0,
                    c0,
                    &format!("`{item}` is private to `{module}`"),
                    &format!("declare it `pub` in `{}`", target.file),
                ))
            }
            Some(_) => out.push_str(&qualified_name(&module, &item)),
        }
    }
    Ok(out)
}

enum LibItem {
    Struct(StructDecl),
    Const(ConstDecl),
    Func(Function),
}

impl LibItem {
    fn name(&self) -> &str {
        match self {
            LibItem::Struct(s) => &s.name,
            LibItem::Const(c) => &c.name,
            LibItem::Func(f) => &f.name,
        }
    }
}

fn type_names(t: &Type, out: &mut Vec<String>) {
    match t {
        Type::Struct(n) => out.push(n.clone()),
        Type::Array(inner, _) => type_names(inner, out),
        _ => {}
    }
}

/// Every identifier an item mentions (bodies, initializers, types).
fn mentions(text: &str, types: &[&Type]) -> Vec<String> {
    let mut v: Vec<String> = identifiers(text).into_iter().map(|(n, _, _)| n).collect();
    for t in types {
        type_names(t, &mut v);
    }
    v
}

fn item_mentions(it: &LibItem) -> Vec<String> {
    match it {
        LibItem::Struct(s) => {
            let tys: Vec<&Type> = s.fields.iter().map(|f| &f.ty).collect();
            mentions("", &tys)
        }
        LibItem::Const(c) => mentions(&c.initializer, &c.ty.iter().collect::<Vec<_>>()),
        LibItem::Func(f) => {
            let mut tys: Vec<&Type> = f.params.iter().map(|p| &p.ty).collect();
            tys.extend(f.return_type.iter());
            mentions(&f.body, &tys)
        }
    }
}

fn module_mentions(m: &ShaderModule) -> Vec<String> {
    let mut v = Vec::new();
    for s in &m.structs {
        v.extend(item_mentions(&LibItem::Struct(s.clone())));
    }
    for c in &m.const_decls {
        v.extend(item_mentions(&LibItem::Const(c.clone())));
    }
    for f in &m.functions {
        v.extend(item_mentions(&LibItem::Func(f.clone())));
    }
    for b in &m.bindings {
        let mut tys = Vec::new();
        match &b.kind {
            BindingKind::Uniform(t) => tys.push(t),
            BindingKind::Storage { ty, .. } => tys.push(ty),
            _ => {}
        }
        v.extend(mentions("", &tys));
    }
    for p in &m.private_vars {
        v.extend(mentions("", &[&p.ty]));
    }
    for e in &m.entry_points {
        let mut tys: Vec<&Type> = e.params.iter().map(|p| &p.ty).collect();
        tys.extend(e.return_type.iter().map(|r| &r.ty));
        v.extend(mentions(&e.body, &tys));
    }
    v
}

/// Build and emit the single WGSL module for an entry file.
///
/// `source` is the entry text, `dir` its folder (the `crate` root), `stem` its module name when known
/// (it lets a cycle back to the entry be reported by name).
pub fn transpile(source: &str, dir: &Path, stem: Option<&str>) -> Result<String> {
    let entry_key = stem.unwrap_or("<entry>").to_string();
    let entry_file = stem.map(|s| format!("{s}.wjsl")).unwrap_or_else(|| "<input>".to_string());
    let entry = load_module(dir, &entry_key, &entry_file, source, true)?;
    let mut g = Graph { mods: BTreeMap::new(), order: Vec::new(), entry: entry_key.clone() };
    g.mods.insert(entry_key.clone(), entry);
    visit(dir, &mut g, &mut Vec::new(), &entry_key)?;

    // Resolve, rewrite and parse every module separately.
    let mut parsed: HashMap<String, ShaderModule> = HashMap::new();
    let mut entry_text = String::new();
    for key in &g.order {
        let res = resolve_imports(&g, key)?;
        let text = rewrite(&g.mods[key].scan.text, &g, key, &res)?;
        let ast = if *key == g.entry {
            let ast = crate::wjsl::parse(&text)?;
            entry_text = text;
            ast
        } else {
            let m = &g.mods[key];
            let mut ast = crate::wjsl::parser::parse_wjsl_with_filename(&text, m.file.clone())
                .map_err(|e| ShaderDiagnostics::new(&text).enrich(e))?;
            for s in &mut ast.structs {
                s.origin = origin_of(m, &s.name);
            }
            for c in &mut ast.const_decls {
                c.origin = origin_of(m, &c.name);
            }
            for f in &mut ast.functions {
                f.origin = origin_of(m, &f.name);
            }
            ast
        };
        parsed.insert(key.clone(), ast);
    }

    // Merge: library items in module order, then the entry file.
    let entry_ast = parsed.remove(&g.entry).expect("entry parsed");
    let mut lib: Vec<LibItem> = Vec::new();
    for key in g.order.iter().filter(|k| **k != g.entry) {
        let m = parsed.remove(key).expect("module parsed");
        lib.extend(m.structs.into_iter().map(LibItem::Struct));
        lib.extend(m.const_decls.into_iter().map(LibItem::Const));
        lib.extend(m.functions.into_iter().map(LibItem::Func));
    }
    let assemble = |lib: &[&LibItem], e: &ShaderModule| {
        let mut m = ShaderModule {
            structs: Vec::new(),
            bindings: e.bindings.clone(),
            private_vars: e.private_vars.clone(),
            const_decls: Vec::new(),
            functions: Vec::new(),
            entry_points: e.entry_points.clone(),
        };
        for it in lib {
            match it {
                LibItem::Struct(s) => m.structs.push(s.clone()),
                LibItem::Const(c) => m.const_decls.push(c.clone()),
                LibItem::Func(f) => m.functions.push(f.clone()),
            }
        }
        m.structs.extend(e.structs.iter().cloned());
        m.const_decls.extend(e.const_decls.iter().cloned());
        m.functions.extend(e.functions.iter().cloned());
        m
    };

    // The checker sees every library item (unreached library code is still checked) ...
    let all: Vec<&LibItem> = lib.iter().collect();
    let full = assemble(&all, &entry_ast);
    crate::wjsl::check(&full, &entry_text)?;

    // ... and the emitter only what the entry file reaches (dead-code elimination).
    let table: HashMap<&str, usize> = lib.iter().enumerate().map(|(i, it)| (it.name(), i)).collect();
    let mut reached: BTreeSet<usize> = BTreeSet::new();
    let mut work: Vec<usize> = Vec::new();
    for name in module_mentions(&entry_ast) {
        if let Some(&ix) = table.get(name.as_str()) {
            if reached.insert(ix) {
                work.push(ix);
            }
        }
    }
    while let Some(ix) = work.pop() {
        for name in item_mentions(&lib[ix]) {
            if let Some(&j) = table.get(name.as_str()) {
                if reached.insert(j) {
                    work.push(j);
                }
            }
        }
    }
    let kept: Vec<&LibItem> = reached.iter().map(|&ix| &lib[ix]).collect();
    let pruned = assemble(&kept, &entry_ast);
    crate::wjsl::emit_wgsl(pruned, &entry_text)
}

/// `brdf.wjsl:2` for the library item whose emitted name is `qualified`.
fn origin_of(m: &ModuleSrc, qualified: &str) -> Option<String> {
    m.scan
        .items
        .iter()
        .find(|i| qualified_name(&m.stem, &i.name) == qualified)
        .map(|i| format!("{}:{}", m.file, i.line))
}
