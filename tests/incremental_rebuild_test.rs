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

#[test]
fn submodule_mod_decl_before_parent_and_importers() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(src.join("domain")).unwrap();
    fs::create_dir_all(src.join("adapters")).unwrap();

    let domain_mod = src.join("domain/mod.wj");
    let domain_render = src.join("domain/render.wj");
    let adapters_fs = src.join("adapters/fs_site.wj");
    fs::write(&domain_mod, "pub mod render\npub use render::generate_page\n").unwrap();
    fs::write(&domain_render, "pub fn generate_page(path: string, markdown: string) -> string { markdown }\n").unwrap();
    fs::write(&adapters_fs, "use crate::domain::generate_page\npub fn load(path: string, md: string) -> string { generate_page(path, md) }\n").unwrap();

    // Alphabetical discovery would put adapters before domain/render — sort must still
    // codegen render before fs_site.
    let sources = vec![
        (adapters_fs.clone(), fs::read_to_string(&adapters_fs).unwrap()),
        (domain_mod.clone(), fs::read_to_string(&domain_mod).unwrap()),
        (domain_render.clone(), fs::read_to_string(&domain_render).unwrap()),
    ];
    let mut parsers = Vec::new();
    let mut programs = Vec::new();
    for (file, source) in &sources {
        let (parser, program) = parse_file(file, source);
        parsers.push(parser);
        programs.push(program);
    }
    let _keep = parsers;
    let graph = DependencyGraph::build(&sources, &programs, &src);
    let sorted = graph.sort_indices_for_codegen(&[0, 1, 2]);
    let render_pos = sorted.iter().position(|&i| i == 2).expect("render");
    let mod_pos = sorted.iter().position(|&i| i == 1).expect("mod");
    let fs_pos = sorted.iter().position(|&i| i == 0).expect("fs");
    assert!(
        render_pos < mod_pos,
        "domain/mod re-exports render — render must codegen first: {:?}",
        sorted
    );
    assert!(
        render_pos < fs_pos,
        "fs_site imports generate_page — defining render must precede fs_site: {:?}",
        sorted
    );
}

/// Re-exported item imports (`use crate::graph::Port`) must depend on the *defining*
/// submodule, not the package `mod.wj`. Otherwise `mod → child` + `child → mod` cycles
/// collapse Kahn sort to discovery order (session before engine → stale call-site borrows).
#[test]
/// Dogfood shape: package `mod.wj` lists session before engine alphabetically, while
/// session imports the engine free-fn. Codegen must still emit engine before session.
/// Mutual imports form an SCC; the callee with more pending importers must sort first.
#[test]
fn codegen_sort_breaks_scc_preferring_most_depended_upon() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    let a = src.join("a.wj");
    let b = src.join("b.wj");
    let c = src.join("c.wj");
    fs::write(&a, "use crate::b::b_fn\npub fn a_fn() { b_fn() }\n").unwrap();
    fs::write(&b, "use crate::a::a_fn\npub fn b_fn() { a_fn() }\n").unwrap();
    fs::write(&c, "use crate::b::b_fn\npub fn c_fn() { b_fn() }\n").unwrap();
    let sources = vec![
        (a.clone(), fs::read_to_string(&a).unwrap()),
        (b.clone(), fs::read_to_string(&b).unwrap()),
        (c.clone(), fs::read_to_string(&c).unwrap()),
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
    let sorted = graph.sort_indices_for_codegen(&[0, 1, 2]);
    let b_pos = sorted.iter().position(|&i| i == 1).unwrap();
    let a_pos = sorted.iter().position(|&i| i == 0).unwrap();
    let c_pos = sorted.iter().position(|&i| i == 2).unwrap();
    assert!(
        b_pos < a_pos && b_pos < c_pos,
        "b (depended on by a and c) before importers, got {:?}",
        sorted
    );
}

/// Live dogfood check (skipped if wdb-layers is not checked out beside windjammer).
#[test]
fn wdb_layers_graph_codegen_order_session_after_bfs() {
    let layers_src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../windjammerdb/crates/wdb-layers/src");
    if !layers_src.join("graph/graph_analytics_session.wj").is_file() {
        eprintln!("skip: wdb-layers not present at {}", layers_src.display());
        return;
    }
    let mut sources: Vec<(std::path::PathBuf, String)> = Vec::new();
    fn walk(dir: &std::path::Path, out: &mut Vec<(std::path::PathBuf, String)>) {
        let Ok(rd) = fs::read_dir(dir) else {
            return;
        };
        for ent in rd.flatten() {
            let p = ent.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().and_then(|s| s.to_str()) == Some("wj") {
                out.push((p.clone(), fs::read_to_string(&p).unwrap()));
            }
        }
    }
    walk(&layers_src, &mut sources);
    sources.sort_by(|a, b| a.0.cmp(&b.0));
    let mut parsers = Vec::new();
    let mut programs = Vec::new();
    for (file, source) in &sources {
        let (parser, program) = parse_file(file, source);
        parsers.push(parser);
        programs.push(program);
    }
    let _ = parsers;
    let graph = DependencyGraph::build(&sources, &programs, &layers_src);
    let indices: Vec<usize> = (0..sources.len()).collect();
    let sorted = graph.sort_indices_for_codegen(&indices);
    let pos = |suffix: &str| {
        let idx = sources
            .iter()
            .position(|(p, _)| p.ends_with(suffix))
            .unwrap_or_else(|| panic!("missing {suffix}"));
        sorted.iter().position(|&i| i == idx).expect("in sorted")
    };
    let session_pos = pos("graph_analytics_session.wj");
    let bfs_pos = pos("graph_bfs_engine.wj");
    let df_pos = pos("graph_sql_datafusion_port.wj");
    let phys_pos = pos("graph_sql_physical_exec_port.wj");
    assert!(
        bfs_pos < session_pos,
        "bfs@{bfs_pos} must codegen before session@{session_pos}"
    );
    assert!(
        phys_pos < df_pos,
        "physical_exec@{phys_pos} must codegen before datafusion@{df_pos}"
    );
}

#[test]
fn package_mod_decl_order_must_not_codegen_importer_before_callee() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(src.join("graph")).unwrap();

    let graph_mod = src.join("graph/mod.wj");
    // Intentionally list session before bfs (matches wdb-layers graph/mod.wj order).
    fs::write(
        &graph_mod,
        "pub mod graph_analytics_session\npub mod graph_bfs_engine\npub mod graph_dense_csr\n",
    )
    .unwrap();
    let dense = src.join("graph/graph_dense_csr.wj");
    fs::write(&dense, "pub struct DenseCsr { pub n: int }\n").unwrap();
    let bfs = src.join("graph/graph_bfs_engine.wj");
    fs::write(
        &bfs,
        "use crate::graph::graph_dense_csr::DenseCsr\npub fn graph_bfs_run_dense(csr: DenseCsr, source: int) -> int { csr.n + source }\n",
    )
    .unwrap();
    let session = src.join("graph/graph_analytics_session.wj");
    fs::write(
        &session,
        "use crate::graph::graph_dense_csr::DenseCsr\nuse crate::graph::graph_bfs_engine::graph_bfs_run_dense\npub fn go(csr: DenseCsr) -> int { graph_bfs_run_dense(csr, 1) }\n",
    )
    .unwrap();

    // Discovery order mirrors alphabetical / mod.wj listing: session before bfs.
    let sources = vec![
        (session.clone(), fs::read_to_string(&session).unwrap()),
        (graph_mod.clone(), fs::read_to_string(&graph_mod).unwrap()),
        (dense.clone(), fs::read_to_string(&dense).unwrap()),
        (bfs.clone(), fs::read_to_string(&bfs).unwrap()),
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
    let deps = graph.depends_on_for_tests();
    assert!(
        deps.get(&0).is_some_and(|d| d.contains(&3)),
        "session must depend on bfs_engine, deps={:?}",
        deps.get(&0)
    );
    let sorted = graph.sort_indices_for_codegen(&[0, 1, 2, 3]);
    let session_pos = sorted.iter().position(|&i| i == 0).unwrap();
    let bfs_pos = sorted.iter().position(|&i| i == 3).unwrap();
    let dense_pos = sorted.iter().position(|&i| i == 2).unwrap();
    assert!(
        dense_pos < bfs_pos && bfs_pos < session_pos,
        "expected dense → bfs → session, got {:?}",
        sorted
    );
}

#[test]
fn reexport_type_import_depends_on_defining_module_not_package_mod() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(src.join("graph")).unwrap();

    let graph_mod = src.join("graph/mod.wj");
    let types = src.join("graph/types.wj");
    let engine = src.join("graph/engine.wj");
    let session = src.join("graph/session.wj");
    fs::write(
        &graph_mod,
        "pub mod types\npub mod engine\npub mod session\npub use types::Port\n",
    )
    .unwrap();
    fs::write(&types, "pub struct Port { pub n: int }\n").unwrap();
    fs::write(
        &engine,
        "use crate::graph::Port\npub fn run(p: Port) -> int { p.n }\n",
    )
    .unwrap();
    fs::write(
        &session,
        "use crate::graph::types::Port\nuse crate::graph::engine::run\npub fn go(p: Port) -> int { run(p) }\n",
    )
    .unwrap();

    // Discovery order puts session before engine alphabetically within graph/.
    let sources = vec![
        (session.clone(), fs::read_to_string(&session).unwrap()),
        (engine.clone(), fs::read_to_string(&engine).unwrap()),
        (graph_mod.clone(), fs::read_to_string(&graph_mod).unwrap()),
        (types.clone(), fs::read_to_string(&types).unwrap()),
    ];
    let mut parsers = Vec::new();
    let mut programs = Vec::new();
    for (file, source) in &sources {
        let (parser, program) = parse_file(file, source);
        parsers.push(parser);
        programs.push(program);
    }
    let _keep = parsers;
    let graph = DependencyGraph::build(&sources, &programs, &src);
    let sorted = graph.sort_indices_for_codegen(&[0, 1, 2, 3]);
    let session_pos = sorted.iter().position(|&i| i == 0).expect("session");
    let engine_pos = sorted.iter().position(|&i| i == 1).expect("engine");
    let types_pos = sorted.iter().position(|&i| i == 3).expect("types");
    assert!(
        types_pos < engine_pos && engine_pos < session_pos,
        "expected types → engine → session (no mod.wj cycle), got {:?}",
        sorted
    );
}
