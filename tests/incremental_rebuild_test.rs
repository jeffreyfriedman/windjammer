//! Incremental dependency graph and reanalysis set tests.

use std::collections::HashSet;
use std::fs;
use tempfile::TempDir;
use windjammer::compiler::cache_management;
use windjammer::compiler::incremental::{compute_reanalysis_set, DependencyGraph};
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

fn parse_file(path: &std::path::Path, source: &str) -> (Parser, windjammer::parser::Program<'static>) {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new_with_source(
        tokens,
        path.to_string_lossy().to_string(),
        source.to_string(),
    );
    let program = parser.parse().expect("parse test fixture");
    (parser, program)
}

#[test]
fn test_dependency_graph_transitive_dependents() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();

    let a = src.join("a.wj");
    let b = src.join("b.wj");
    fs::write(&a, "fn a_fn() {}\n").unwrap();
    fs::write(&b, "use crate::a;\nfn b_fn() {}\n").unwrap();

    let sources = vec![
        (a.clone(), fs::read_to_string(&a).unwrap()),
        (b.clone(), fs::read_to_string(&b).unwrap()),
    ];
    let mut parsers = Vec::new();
    let mut programs = Vec::new();
    for (file, source) in &sources {
        let (parser, program) = parse_file(file, source);
        parsers.push(parser);
        programs.push(program);
    }
    let _ = parsers;

    let graph = DependencyGraph::build(&sources, &programs, &src);
    let mut dirty = HashSet::new();
    dirty.insert(0);
    let dependents = graph.transitive_dependents(&dirty);
    assert!(dependents.contains(&0));
}

#[test]
fn test_dependency_graph_type_import_depends_on_defining_module() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();

    let squad = src.join("squad.wj");
    let caller = src.join("caller.wj");
    fs::write(
        &squad,
        r#"pub struct Squad { id: string }
impl Squad {
    pub fn new(id: string) -> Squad { Squad { id: id } }
}
"#,
    )
    .unwrap();
    fs::write(
        &caller,
        r#"use squad::Squad
pub fn make_squad(id: string) -> Squad { Squad::new(id) }
"#,
    )
    .unwrap();

    let sources = vec![
        (caller.clone(), fs::read_to_string(&caller).unwrap()),
        (squad.clone(), fs::read_to_string(&squad).unwrap()),
    ];
    let mut programs = Vec::new();
    for (file, source) in &sources {
        let (_, program) = parse_file(file, source);
        programs.push(program);
    }

    let graph = DependencyGraph::build(&sources, &programs, &src);
    let sorted = graph.sort_indices_for_codegen(&[0, 1]);
    assert_eq!(
        sorted,
        vec![1, 0],
        "defining module squad must codegen before importer caller; got {:?}",
        sorted
    );
}

#[test]
fn test_dependency_graph_super_import_depends_on_sibling_module() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();

    let table = src.join("table.wj");
    let datatable = src.join("datatable.wj");
    fs::write(
        &table,
        r#"pub struct Table { n: int }
impl Table {
    pub fn bump(self) -> Table { self }
}
"#,
    )
    .unwrap();
    fs::write(
        &datatable,
        r#"use super::table::Table
pub struct DataTable { table: Table }
impl DataTable {
    pub fn bump(self) -> DataTable {
        self.table = self.table.bump()
        self
    }
}
"#,
    )
    .unwrap();

    // Importer listed first so index order alone would put datatable before table.
    let sources = vec![
        (datatable.clone(), fs::read_to_string(&datatable).unwrap()),
        (table.clone(), fs::read_to_string(&table).unwrap()),
    ];
    let mut programs = Vec::new();
    for (file, source) in &sources {
        let (_, program) = parse_file(file, source);
        programs.push(program);
    }

    let graph = DependencyGraph::build(&sources, &programs, &src);
    let sorted = graph.sort_indices_for_codegen(&[0, 1]);
    assert_eq!(
        sorted,
        vec![1, 0],
        "table must codegen before datatable importer via super::table; got {:?}",
        sorted
    );
}

#[test]
fn test_dependency_graph_braced_crate_import_depends_on_defining_module() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();

    let analytics = src.join("analytics.wj");
    let routes = src.join("routes.wj");
    fs::write(
        &analytics,
        r#"pub struct AppDeps { n: int }
pub fn create_export_job(mut deps: AppDeps) -> int {
    deps.n = deps.n + 1
    deps.n
}
"#,
    )
    .unwrap();
    fs::write(
        &routes,
        r#"use crate::analytics::{create_export_job, AppDeps}
pub fn handler(deps: AppDeps) -> int {
    create_export_job(deps)
}
"#,
    )
    .unwrap();

    // Importer listed first so index order alone would put routes before analytics.
    let sources = vec![
        (routes.clone(), fs::read_to_string(&routes).unwrap()),
        (analytics.clone(), fs::read_to_string(&analytics).unwrap()),
    ];
    let mut programs = Vec::new();
    for (file, source) in &sources {
        let (_, program) = parse_file(file, source);
        programs.push(program);
    }

    let graph = DependencyGraph::build(&sources, &programs, &src);
    let sorted = graph.sort_indices_for_codegen(&[0, 1]);
    assert_eq!(
        sorted,
        vec![1, 0],
        "braced `use crate::analytics::{{…}}` must depend on analytics; got {:?}",
        sorted
    );
}

#[test]
fn test_compute_reanalysis_set_all_dirty_without_meta() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    let wj = src.join("x.wj");
    fs::write(&wj, "fn x() {}\n").unwrap();
    cache_management::write_compiler_stamp(dir.path()).unwrap();

    let sources = vec![(wj, "fn x() {}\n".to_string())];
    let mut parsers = Vec::new();
    let mut programs = Vec::new();
    for (file, source) in &sources {
        let (parser, program) = parse_file(file, source);
        parsers.push(parser);
        programs.push(program);
    }
    let _ = parsers;

    let graph = DependencyGraph::build(&sources, &programs, &src);
    let set = compute_reanalysis_set(&sources, &src, dir.path(), 0, &graph);
    assert_eq!(set.len(), 1);
}
