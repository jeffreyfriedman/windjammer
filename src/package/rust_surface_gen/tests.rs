//! Generator tests on a fixture crate written here (independent oracle: the expectations are stated by hand
//! from the Rust source, not computed by the generator).

use super::*;
use std::collections::BTreeSet;
use std::path::Path;

const LIB: &str = r#"
//! fixture
mod inner;
mod hidden;
pub mod util;
pub use inner::{shout, quiet as whisper};
pub use util::*;
pub use other_crate::foreign;
pub type Count = u64;

/// Adds two numbers.
/// More text.
pub fn add(a: i32, b: i32) -> i32 { a + b }
pub fn count(s: &str) -> Count { s.len() as u64 }
pub fn bytes_len(b: &[u8], out: &mut Vec<u8>) -> usize { b.len() + out.len() }
pub fn maybe(x: Option<String>) -> Result<Vec<String>, String> { Ok(vec![]) }
pub fn first<T: Clone>(v: &[T]) -> T { v[0].clone() }
pub fn show(x: impl std::fmt::Display) -> String { x.to_string() }
pub fn borrow<'a>(s: &'a str) -> &'a str { s }
pub fn takes_string(s: String) -> usize { s.len() }
pub async fn later() {}
pub unsafe fn raw() {}
#[doc(hidden)]
pub fn internal_helper() {}
#[cfg(feature = "extra")]
pub fn extra() -> bool { true }
#[cfg(feature = "std")]
pub fn with_std() -> bool { true }
#[cfg(unix)]
pub fn unix_only() -> bool { true }
#[cfg(test)]
pub fn test_only() {}
fn private() {}
pub(crate) fn crate_only() {}
pub struct Thing;
impl Thing { pub fn method(&self) -> usize { 0 } }
pub fn make() -> Thing { Thing }
some_macro!(generated_fn);
macro_rules! local { () => {} }
"#;

fn fixture(dir: &Path) {
    std::fs::create_dir_all(dir.join("src/util")).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"1.2.3\"\n\n[features]\ndefault = [\"std\"]\nstd = [\"alloc\"]\nalloc = []\nextra = []\n",
    )
    .unwrap();
    std::fs::write(dir.join("src/lib.rs"), LIB).unwrap();
    std::fs::write(dir.join("src/inner.rs"), "pub fn shout(s: &str) -> String { s.to_uppercase() }\npub fn quiet(s: &str) -> String { s.to_lowercase() }\npub fn not_reexported() {}\n").unwrap();
    std::fs::write(dir.join("src/util/mod.rs"), "pub fn twice(n: u32) -> u32 { n * 2 }\nfn not_public() {}\n").unwrap();
}

fn source(dir: &Path, features: &[&str]) -> CrateSource {
    let entry = locate::DepEntry {
        key: "fixture".into(),
        package: "fixture".into(),
        features: features.iter().map(|s| s.to_string()).collect(),
        default_features: true,
        ..Default::default()
    };
    locate::describe(dir, "fixture", &entry).unwrap()
}

fn generated(features: &[&str]) -> Generated {
    let tmp = tempfile::TempDir::new().unwrap();
    fixture(tmp.path());
    generate(&source(tmp.path(), features)).unwrap()
}

fn sig(g: &Generated, name: &str) -> String {
    let f = g.fns.iter().find(|f| f.name == name).unwrap_or_else(|| panic!("{name} not generated: {:?}", g.skipped));
    let p: Vec<String> = f.params.iter().map(|(n, t)| format!("{n}: {t}")).collect();
    format!("({}){}", p.join(", "), f.ret.as_ref().map(|r| format!(" -> {r}")).unwrap_or_default())
}

fn reason<'a>(g: &'a Generated, name: &str) -> &'a str {
    &g.skipped.iter().find(|(n, _)| n == name).unwrap_or_else(|| panic!("{name} not skipped: {:?}", g.skipped)).1
}

#[test]
fn features_expand_defaults() {
    let tmp = tempfile::TempDir::new().unwrap();
    fixture(tmp.path());
    let s = source(tmp.path(), &[]);
    assert_eq!(s.features, ["alloc", "default", "std"].iter().map(|s| s.to_string()).collect::<BTreeSet<_>>());
    assert_eq!(s.version, "1.2.3");
}

#[test]
fn mappable_functions_are_declared() {
    let g = generated(&[]);
    assert_eq!(sig(&g, "add"), "(a: i32, b: i32) -> i32");
    assert_eq!(sig(&g, "count"), "(s: string) -> u64");
    assert_eq!(sig(&g, "bytes_len"), "(b: &Vec<u8>, out: &mut Vec<u8>) -> usize");
    assert_eq!(sig(&g, "maybe"), "(x: Option<string>) -> Result<Vec<string>, string>");
    assert_eq!(sig(&g, "shout"), "(s: string) -> string");
    assert_eq!(sig(&g, "whisper"), "(s: string) -> string");
    assert_eq!(sig(&g, "twice"), "(n: u32) -> u32");
    assert_eq!(sig(&g, "with_std"), "() -> bool");
    assert_eq!(g.fns.iter().find(|f| f.name == "add").unwrap().doc.as_deref(), Some("Adds two numbers."));
}

#[test]
fn every_public_function_is_declared_or_skipped_with_a_reason() {
    let g = generated(&[]);
    assert!(reason(&g, "first").contains("generic over `T: Clone`"));
    assert!(reason(&g, "show").contains("trait-bound"));
    assert!(reason(&g, "borrow").contains("returns a borrow"));
    assert!(reason(&g, "takes_string").contains("owned `String`"));
    assert!(reason(&g, "later").contains("async"));
    assert!(reason(&g, "raw").contains("unsafe"));
    assert!(reason(&g, "internal_helper").contains("doc(hidden)"));
    assert!(reason(&g, "unix_only").contains("cfg(unix)"));
    assert!(reason(&g, "foreign").contains("outside this crate"));
    assert!(reason(&g, "make").contains("crate type `Thing`"));
    // never reachable as `fixture::f`: private, pub(crate), cfg(test), a feature that is off, not re-exported
    let all: BTreeSet<&str> = g.fns.iter().map(|f| f.name.as_str()).chain(g.skipped.iter().map(|(n, _)| n.as_str())).collect();
    for absent in ["private", "crate_only", "test_only", "extra", "not_reexported", "not_public", "method", "quiet"] {
        assert!(!all.contains(absent), "{absent} must not appear: {all:?}");
    }
    assert_eq!(g.fns.len() + g.skipped.len(), 18, "declared {:?}\nskipped {:?}", g.fns, g.skipped);
    assert!(g.notes.iter().any(|n| n.contains("1 macro invocation")), "{:?}", g.notes);
}

#[test]
fn enabled_features_are_honoured() {
    let g = generated(&["extra"]);
    assert_eq!(sig(&g, "extra"), "() -> bool");
}

#[test]
fn unlisted_crates_have_unknown_authority() {
    let g = generated(&[]);
    assert_eq!(g.pure, pure::Verdict::Unlisted);
    let text = render(&g, Mode::Cache, None);
    assert!(!text.contains("@pure"), "{text}");
    assert!(text.contains("every call is ffi(fixture)"), "{text}");
}

#[test]
fn render_header_and_filter() {
    let g = generated(&[]);
    let cache = render(&g, Mode::Cache, None);
    assert!(cache.starts_with(CACHE_MARKER), "{cache}");
    let h = header(&cache).unwrap();
    assert_eq!((h.crate_name.as_str(), h.version.as_str(), h.cache, h.stamp.as_str(), h.line), ("fixture", "1.2.3", true, stamp().as_str(), 3));
    assert!(cache.contains(&format!("// source: sha256:{}", g.source_hash)) && g.source_hash.len() == 64);
    assert!(cache.contains("pub fn add(a: i32, b: i32) -> i32\n"), "{cache}");
    assert!(cache.contains("// skipped first: generic over `T: Clone`"), "{cache}");
    assert_eq!(skipped_in(&cache).len(), g.skipped.len());
    let pinned = render(&g, Mode::Pinned, Some(&["add".to_string()]));
    assert!(!pinned.contains(CACHE_MARKER) && !header(&pinned).unwrap().cache);
    assert!(pinned.contains("pub fn add(") && !pinned.contains("pub fn count("), "{pinned}");
    // the generated text is a valid surface
    let s = crate::package::rust_surface::parse_surface("fixture", "x.wji".into(), cache.clone());
    assert!(s.problems.is_empty(), "{:?}", s.problems);
    assert_eq!(s.fns.len(), g.fns.len());
    assert_eq!(s.skipped.len(), g.skipped.len());
}

#[test]
fn source_hash_tracks_the_source() {
    let tmp = tempfile::TempDir::new().unwrap();
    fixture(tmp.path());
    let a = generate(&source(tmp.path(), &[])).unwrap();
    std::fs::write(tmp.path().join("src/inner.rs"), "pub fn shout(s: &str) -> String { s.into() }\n").unwrap();
    let b = generate(&source(tmp.path(), &[])).unwrap();
    assert_ne!(a.source_hash, b.source_hash);
    // `quiet` is gone, so the re-export names no function (a re-exported type or const is not a function either)
    assert!(!b.fns.iter().any(|f| f.name == "whisper") && !b.skipped.iter().any(|(n, _)| n == "whisper"));
}
