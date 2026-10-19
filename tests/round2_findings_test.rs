#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]
//! Generic regression gates for the second round of compiler findings from a large backend:
//! runtime `&str` formals inside trait impls, untyped counters, call-site borrows across modules, and
//! interpreter parity (same-name private types, inferred JSON targets, request values).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn write_project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (rel, body) in files {
        let p = dir.path().join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }
    dir
}

fn run(dir: &std::path::Path, args: &[&str]) -> (bool, String) {
    let o = Command::new(WJ).args(args).current_dir(dir).output().unwrap();
    (o.status.success(), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)))
}

/// Emitted Rust of a single program.
fn emitted(src: &str) -> String {
    let dir = write_project(&[("main.wj", src)]);
    let (ok, out) = run(dir.path(), &["build", "main.wj", "--no-cargo", "--output", "out"]);
    assert!(ok, "wj build failed:\n{out}");
    fs::read_to_string(dir.path().join("out/main.rs")).unwrap()
}

fn project_compiles(files: &[(&str, &str)]) {
    let dir = write_project(files);
    let out = dir.path().join("gen");
    let o = Command::new(WJ)
        .arg("build")
        .arg(dir.path().join("src/mod.wj"))
        .arg("--output")
        .arg(&out)
        .args(["--library", "--no-cargo", "--module-file"])
        .output()
        .unwrap();
    assert!(o.status.success(), "wj build failed:\n{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    let rustc = Command::new("rustc")
        .args(["--crate-type", "lib", "--edition", "2021", "--emit", "metadata", "-o"])
        .arg(dir.path().join("out.rmeta"))
        .arg(out.join("lib.rs"))
        .output()
        .unwrap();
    assert!(rustc.status.success(), "generated library does not compile:\n{}", String::from_utf8_lossy(&rustc.stderr));
}

// ------------------------------------------------------------------ trait impl + runtime &str formal

/// An owned `string` formal of a trait-impl method (the trait keeps owned formals) passed to a runtime
/// `&str` formal must be borrowed, exactly like in a free function.
#[test]
fn trait_impl_owned_string_param_is_borrowed_for_runtime_str_formal() {
    let rs = emitted(
        r#"use std::crypto

trait Authenticator {
    fn check(self, user: string, password: string) -> bool
}

struct Stored {}

impl Authenticator for Stored {
    fn check(self, user: string, password: string) -> bool {
        if user.is_empty() {
            return false
        }
        let hash = match crypto.hash_password("secret") {
            Ok(h) => h,
            Err(_) => return false,
        }
        match crypto.verify_password(password, hash) {
            Ok(value) => value,
            Err(_) => false,
        }
    }
}

fn main() {
    let s = Stored {}
    println(s.check("a", "secret"))
}
"#,
    );
    assert!(rs.contains("crypto::verify_password(&password, &hash)"), "owned trait-impl param not borrowed:\n{rs}");
}

// ------------------------------------------------------------------ untyped counter vs len()

/// Acceptance test (numeric solver, ADR-0015): `let mut i = 0` compared with a length and then passed
/// where `int` is required is one type (`int`); the program must type-check as it runs interpreted.
#[test]
fn untyped_counter_compared_with_len_meets_int_formal() {
    let dir = write_project(&[(
        "main.wj",
        r#"use std::strings

fn char_at(s: string, i: int) -> string {
    strings.substring(s, i, i + 1)
}

fn first_digit_count(s: string) -> int {
    let n = strings.len(s)
    let mut i = 0
    let mut count = 0
    while i < n {
        if char_at(s, i) == "1" { count = count + 1 }
        i = i + 1
    }
    count
}

fn main() { println(first_digit_count("1011")) }
"#,
    )]);
    let (ok, out) = run(dir.path(), &["run", "--interpret", "main.wj"]);
    assert!(ok && out.trim() == "3", "interpreted run failed:\n{out}");
    let (ok, out) = run(dir.path(), &["build", "main.wj", "--check"]);
    assert!(ok, "type check of the compiled form failed:\n{out}");
}

// ------------------------------------------------------------------ call-site borrows across modules (guards)

/// Guard: read-only struct formals defined in one module, called from another with owned values, struct
/// literals built from reused fields, and `.clone()` arguments. The original failure (`expected AppDeps,
/// found &AppDeps`) appeared only in a large project and was not reproduced in minimal form.
#[test]
fn call_site_borrows_agree_with_callee_formals_across_modules() {
    project_compiles(&[
        (
            "src/claims.wj",
            "pub struct AuthClaims {\n    pub email: string,\n    pub tenant: string,\n    pub roles: Vec<string>,\n}\n\npub fn actor_email_from_claims(claims: AuthClaims) -> string {\n    claims.email\n}\n\npub fn tenant_slug_from_claims(claims: AuthClaims) -> string {\n    claims.tenant\n}\n\npub struct OperationContext {\n    pub actor: string,\n    pub tenant: string,\n}\n\npub fn operation_context_valid(ctx: OperationContext) -> bool {\n    !ctx.actor.is_empty() && !ctx.tenant.is_empty()\n}\n",
        ),
        (
            "src/authenticate.wj",
            "use crate::claims::AuthClaims\nuse crate::claims::OperationContext\nuse crate::claims::actor_email_from_claims\nuse crate::claims::tenant_slug_from_claims\nuse crate::claims::operation_context_valid\n\npub fn build(email: string, tenant: string, roles: Vec<string>) -> string {\n    let claims_for_email = AuthClaims { email: email, tenant: tenant, roles: roles }\n    let actor = actor_email_from_claims(claims_for_email)\n    let slug = tenant_slug_from_claims(AuthClaims { email: email, tenant: tenant, roles: roles })\n    actor + slug\n}\n\npub fn post(ctx: OperationContext) -> bool {\n    if operation_context_valid(ctx.clone()) {\n        return operation_context_valid(ctx)\n    }\n    false\n}\n",
        ),
        ("src/mod.wj", "pub mod claims\npub mod authenticate\n"),
    ]);
}

/// Guard: mutually recursive modules passing one read-only struct to several callees.
#[test]
fn shared_struct_passed_to_several_callees_in_cyclic_modules() {
    project_compiles(&[
        ("src/deps.wj", "pub struct AppDeps {\n    pub name: string,\n    pub limit: int,\n    pub tags: Vec<string>,\n}\n\npub struct Tenant {\n    pub slug: string,\n}\n"),
        (
            "src/items.wj",
            "use crate::deps::AppDeps\nuse crate::deps::Tenant\nuse crate::routes::describe\n\npub fn create_item(deps: AppDeps, tenant: string, n: int) -> string {\n    let d = describe(deps, n)\n    tenant + \":\" + d\n}\n\npub fn list_items(deps: AppDeps, tenant: Tenant) -> int {\n    deps.limit + tenant.slug.len() as int\n}\n\npub fn count_tags(deps: AppDeps) -> int {\n    deps.tags.len() as int\n}\n",
        ),
        (
            "src/routes.wj",
            "use crate::deps::AppDeps\nuse crate::deps::Tenant\nuse crate::items::create_item\nuse crate::items::list_items\nuse crate::items::count_tags\n\npub fn describe(deps: AppDeps, n: int) -> string {\n    deps.name + \"-\" + n.to_string()\n}\n\npub fn handle(deps: AppDeps, slug: string) -> string {\n    let first = create_item(deps, slug, 1)\n    let total = list_items(deps, Tenant { slug: slug })\n    let tags = count_tags(deps)\n    first + total.to_string() + tags.to_string()\n}\n",
        ),
        ("src/mod.wj", "pub mod deps\npub mod items\npub mod routes\n"),
    ]);
}

// ------------------------------------------------------------------ interpreter parity

fn interpreted_project(files: &[(&str, &str)]) -> String {
    let dir = write_project(files);
    let (ok, out) = run(dir.path(), &["run", "--interpret", "main.wj"]);
    assert!(ok, "interpreted run failed:\n{out}");
    out
}

/// Two modules each define a private `struct Env` with different fields: legal compiled, and the
/// interpreter must run it (it keys types by name; the loader gives each private type its own name).
#[test]
fn same_name_private_structs_in_two_modules_run_interpreted() {
    let files = [
        ("a.wj", "struct Env {\n    x: int,\n}\n\npub fn from_a() -> int {\n    let e = Env { x: 4 }\n    e.x * 2\n}\n"),
        ("b.wj", "struct Env {\n    name: string,\n    n: int,\n}\n\npub fn from_b() -> string {\n    let e = Env { name: \"b\", n: 3 }\n    e.name + e.n.to_string()\n}\n"),
        ("main.wj", "mod a\nmod b\nuse a::from_a\nuse b::from_b\n\nstruct Env {\n    flag: bool,\n}\n\nfn main() {\n    let e = Env { flag: true }\n    println(from_a())\n    println(from_b())\n    println(e.flag)\n}\n"),
    ];
    assert_eq!(interpreted_project(&files), "8\nb3\ntrue\n");
}

/// `json.parse_string` whose target type comes from how the result is used later.
#[test]
fn json_parse_target_type_inferred_from_later_use() {
    let out = interpreted_project(&[(
        "main.wj",
        r#"use std::json

struct Payload { name: string, count: int }
struct Draft { name: string, count: int }

fn to_draft(p: Payload) -> Draft { Draft { name: p.name, count: p.count } }

fn parse(body: string) -> Result<Draft, string> {
    let payload = match json.parse_string(body) {
        Ok(value) => value,
        Err(msg) => return Err(msg),
    }
    Ok(to_draft(payload))
}

fn main() {
    match parse("{\"name\": \"a\", \"count\": 3}") {
        Ok(d) => println("${d.name} ${d.count}"),
        Err(e) => println("err ${e}"),
    }
}
"#,
    )]);
    assert_eq!(out, "a 3\n");
}

/// `std::http::ServerRequest` is a plain struct in the runtime: tests construct it with a literal.
#[test]
fn server_request_literal_is_constructible_interpreted() {
    let out = interpreted_project(&[(
        "main.wj",
        r#"use std::http::ServerRequest
use std::http::HttpMethod

fn describe(req: ServerRequest) -> string {
    req.path + " " + req.body
}

fn main() {
    let req = ServerRequest { method: HttpMethod::GET, path: "/health", query: std::collections::HashMap::new(), headers: [], body: "ok" }
    println(describe(req))
    println(req.header("x").is_none())
}
"#,
    )]);
    assert_eq!(out, "/health ok\ntrue\n");
}

// ------------------------------------------------------------------ numeric / ownership acceptance (downstream reductions)

/// Acceptance test (numeric solver): an untyped literal pushed to a local `Vec<i32>` takes the local's
/// element width, not the enclosing function's `Vec<u32>` return type.
#[test]
fn push_literal_takes_local_vec_width_not_return_type() {
    let rs = emitted(
        "pub fn f() -> Vec<u32> {\n    let mut a: Vec<i32> = Vec::new()\n    a.push(0)\n    let mut r: Vec<u32> = Vec::new()\n    r.push(1u32)\n    r\n}\n",
    );
    assert!(rs.contains("a.push(0_i32)"), "literal took the wrong width:\n{rs}");
}

/// Acceptance test (ownership, convergence): a `string` parameter that is cloned/moved on one path is
/// still borrowed (`&path`) when passed to a `&str` formal later.
#[test]
fn string_param_borrowed_for_str_formal_after_clone_use() {
    let rs = emitted(
        r#"pub enum Kind {
    Known,
    Unknown(string),
}

pub struct Asset {
    pub path: string,
    pub size: usize,
}

pub struct Loader {
    max: usize,
    loaded: Vec<Asset>,
}

impl Loader {
    fn detect(path: string) -> Result<Kind, string> {
        if path.ends_with(".bin") {
            Ok(Kind::Known)
        } else {
            Err("unsupported")
        }
    }

    fn check(self, size: usize, path: string) -> Result<(), string> {
        if size > self.max {
            Err("${path} is too large")
        } else {
            Ok(())
        }
    }

    pub fn load(self, path: string, size: usize) -> Result<usize, string> {
        let kind = match Loader::detect(path) {
            Ok(k) => k,
            Err(_) => Kind::Unknown(path),
        }
        self.check(size, path)?
        let asset = Asset { path: path, size: size }
        self.loaded.push(asset)
        Ok(size)
    }
}
"#,
    );
    assert!(rs.contains("self.check(size, &path)?"), "owned String passed to a &str formal:\n{rs}");
}
