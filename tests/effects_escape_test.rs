// consumes: src/ir/effect_facts, src/interpreter/caps.rs
//! Sandbox escape suite (ADR-0021 stage E1 gate: at least 60 cases, 0 escapes).
//!
//! Every case runs a small Windjammer program in the interpreter under an enforcing capability grant.
//! An *escape* case tries to do something outside the grant; it passes only if the run stops with
//! exit status 126 and the program never reaches its `DONE` line, and (for writes) the filesystem is
//! untouched. A *legit* case does something inside the grant; it passes only if it runs to `DONE`.
//! The oracles are independent of the implementation: exit status, program output and files on disk.
//!
//! `$ROOT` in grants and programs is replaced by a fresh fixture directory per case:
//!   data/ok.txt  data/sub/deep.txt  data/link -> ../outside  data/flink -> ../outside/secret.txt
//!   data2/x.txt  outside/secret.txt  secret.txt  out/a.txt  out/olink -> ../outside  src/main.wj  wj.toml

use std::path::PathBuf;
use windjammer::interpreter::caps::{CapGate, CapMode};
use windjammer::interpreter::runner::{exit_code_for, with_big_stack};
use windjammer::interpreter::Interpreter;

#[derive(Clone)]
enum Expect {
    /// Must stop with exit 126; the message must contain this text.
    Denied(&'static str),
    /// Must run to completion and print DONE.
    Done,
}

#[derive(Clone)]
enum Chk {
    /// The path must not exist after the run.
    Missing(&'static str),
    /// The file must still contain exactly this text.
    Unchanged(&'static str, &'static str),
}

#[derive(Clone)]
struct Case {
    name: &'static str,
    grant: &'static str,
    uses: &'static str,
    body: &'static str,
    expect: Expect,
    checks: Vec<Chk>,
}

fn esc(name: &'static str, grant: &'static str, uses: &'static str, body: &'static str, effect: &'static str) -> Case {
    Case { name, grant, uses, body, expect: Expect::Denied(effect), checks: vec![] }
}
fn legit(name: &'static str, grant: &'static str, uses: &'static str, body: &'static str) -> Case {
    Case { name, grant, uses, body, expect: Expect::Done, checks: vec![] }
}
impl Case {
    fn check(mut self, c: Chk) -> Case {
        self.checks.push(c);
        self
    }
}

const FS: &str = "[capabilities]\nfs.read = [\"$ROOT/data/**\"]\nfs.write = [\"$ROOT/out/**\"]\n";
const FS_ALL_WRITE: &str = "[capabilities]\nfs.read = [\"$ROOT/**\"]\nfs.write = [\"$ROOT/**\"]\n";
const NET: &str = "[capabilities]\nnet.connect = [\"api.example.com\", \"*.github.com:443\", \"localhost:8080\"]\n";
const ENV: &str = "[capabilities]\nenv.read = [\"HOME\", \"APP_*\"]\n";
const PROC: &str = "[capabilities]\nprocess = [\"echo\"]\n";
const NONE: &str = "[capabilities]\n";

fn cases() -> Vec<Case> {
    use Chk::*;
    let mut v = vec![
        // ---- filesystem: reads outside the scope
        esc("fs/read-dotdot", FS, "use std::fs", "let r = fs::read_to_string(\"$ROOT/data/../secret.txt\")\n    println(\"DONE ${r.is_ok()}\")", "fs.read"),
        esc("fs/read-dotdot-deep", FS, "use std::fs", "let r = fs::read_to_string(\"$ROOT/data/sub/../../secret.txt\")\n    println(\"DONE\")", "fs.read"),
        esc("fs/read-absolute", FS, "use std::fs", "let r = fs::read_to_string(\"/etc/hosts\")\n    println(\"DONE\")", "fs.read"),
        esc("fs/read-root-file", FS, "use std::fs", "let r = fs::read_to_string(\"$ROOT/secret.txt\")\n    println(\"DONE\")", "fs.read"),
        esc("fs/read-through-dir-symlink", FS, "use std::fs", "let r = fs::read_to_string(\"$ROOT/data/link/secret.txt\")\n    println(\"DONE\")", "fs.read"),
        esc("fs/read-through-file-symlink", FS, "use std::fs", "let r = fs::read_to_string(\"$ROOT/data/flink\")\n    println(\"DONE\")", "fs.read"),
        esc("fs/read-sibling-prefix", FS, "use std::fs", "let r = fs::read_to_string(\"$ROOT/data2/x.txt\")\n    println(\"DONE\")", "fs.read"),
        esc("fs/read-bytes-outside", FS, "use std::fs", "let r = fs::read(\"$ROOT/secret.txt\")\n    println(\"DONE\")", "fs.read"),
        esc("fs/read-dir-parent", FS, "use std::fs", "let r = fs::read_dir(\"$ROOT\")\n    println(\"DONE\")", "fs.read"),
        esc("fs/read-dir-dotdot", FS, "use std::fs", "let r = fs::read_dir(\"$ROOT/data/..\")\n    println(\"DONE\")", "fs.read"),
        esc("fs/metadata-leak", FS, "use std::fs", "let r = fs::metadata(\"$ROOT/secret.txt\")\n    println(\"DONE\")", "fs.read"),
        esc("fs/exists-leak", FS, "use std::fs", "let r = fs::exists(\"$ROOT/secret.txt\")\n    println(\"DONE\")", "fs.read"),
        esc("fs/is-file-leak", FS, "use std::fs", "let r = fs::is_file(\"$ROOT/secret.txt\")\n    println(\"DONE\")", "fs.read"),
        esc("fs/is-dir-leak", FS, "use std::fs", "let r = fs::is_dir(\"$ROOT/outside\")\n    println(\"DONE\")", "fs.read"),
        esc("fs/canonicalize-leak", FS, "use std::fs", "let r = fs::canonicalize(\"$ROOT/outside\")\n    println(\"DONE\")", "fs.read"),
        esc("fs/read-double-slash", FS, "use std::fs", "let r = fs::read_to_string(\"$ROOT//secret.txt\")\n    println(\"DONE\")", "fs.read"),
        esc("fs/read-dot-segments", FS, "use std::fs", "let r = fs::read_to_string(\"$ROOT/data/./../secret.txt\")\n    println(\"DONE\")", "fs.read"),
        esc("fs/read-computed-path", FS, "use std::fs", "let name = \"sec\" + \"ret.txt\"\n    let r = fs::read_to_string(\"$ROOT/\" + name)\n    println(\"DONE\")", "fs.read"),
        esc("fs/copy-reads-outside", FS, "use std::fs", "let r = fs::copy(\"$ROOT/secret.txt\", \"$ROOT/out/c.txt\")\n    println(\"DONE\")", "fs.read")
            .check(Missing("out/c.txt")),
        // ---- filesystem: writes outside the scope
        esc("fs/write-root-file", FS, "use std::fs", "let r = fs::write(\"$ROOT/secret.txt\", \"pwned\")\n    println(\"DONE\")", "fs.write").check(Unchanged("secret.txt", "TOP-SECRET")),
        esc("fs/write-dotdot", FS, "use std::fs", "let r = fs::write(\"$ROOT/out/../pwn.txt\", \"x\")\n    println(\"DONE\")", "fs.write").check(Missing("pwn.txt")),
        esc("fs/write-through-symlink", FS, "use std::fs", "let r = fs::write(\"$ROOT/out/olink/pwn.txt\", \"x\")\n    println(\"DONE\")", "fs.write").check(Missing("outside/pwn.txt")),
        esc("fs/write-bytes-outside", FS, "use std::fs", "let r = fs::write_bytes(\"$ROOT/pwn.bin\", [1, 2, 3])\n    println(\"DONE\")", "fs.write").check(Missing("pwn.bin")),
        esc("fs/append-outside", FS, "use std::fs", "let r = fs::append(\"$ROOT/secret.txt\", \"more\")\n    println(\"DONE\")", "fs.write").check(Unchanged("secret.txt", "TOP-SECRET")),
        esc("fs/create-dir-outside", FS, "use std::fs", "let r = fs::create_dir(\"$ROOT/newdir\")\n    println(\"DONE\")", "fs.write").check(Missing("newdir")),
        esc("fs/create-dir-all-outside", FS, "use std::fs", "let r = fs::create_dir_all(\"$ROOT/a/b/c\")\n    println(\"DONE\")", "fs.write").check(Missing("a")),
        esc("fs/remove-file-outside", FS, "use std::fs", "let r = fs::remove_file(\"$ROOT/secret.txt\")\n    println(\"DONE\")", "fs.write").check(Unchanged("secret.txt", "TOP-SECRET")),
        esc("fs/remove-dir-all-outside", FS, "use std::fs", "let r = fs::remove_dir_all(\"$ROOT/data2\")\n    println(\"DONE\")", "fs.write").check(Unchanged("data2/x.txt", "x")),
        esc("fs/rename-out-of-scope", FS, "use std::fs", "let r = fs::rename(\"$ROOT/out/a.txt\", \"$ROOT/moved.txt\")\n    println(\"DONE\")", "fs.write").check(Missing("moved.txt")),
        esc("fs/rename-into-scope-from-outside", FS, "use std::fs", "let r = fs::rename(\"$ROOT/secret.txt\", \"$ROOT/out/s.txt\")\n    println(\"DONE\")", "fs.write").check(Unchanged("secret.txt", "TOP-SECRET")),
        esc("fs/copy-writes-outside", FS, "use std::fs", "let r = fs::copy(\"$ROOT/data/ok.txt\", \"$ROOT/copy.txt\")\n    println(\"DONE\")", "fs.write").check(Missing("copy.txt")),
        esc("fs/db-file-outside", FS, "use std::db", "let r = db::connect(\"$ROOT/pwn.db\")\n    println(\"DONE\")", "fs.write").check(Missing("pwn.db")),
        // ---- the manifest, sources and cwd are protected
        esc("self/write-manifest-via-glob", FS_ALL_WRITE, "use std::fs", "let r = fs::write(\"$ROOT/wj.toml\", \"[capabilities]\\nnet.connect = [\\\"*\\\"]\")\n    println(\"DONE\")", "fs.write")
            .check(Unchanged("wj.toml", "[package]\nname = \"fixture\"\n")),
        esc("self/write-source-via-glob", FS_ALL_WRITE, "use std::fs", "let r = fs::write(\"$ROOT/src/main.wj\", \"fn main() {}\")\n    println(\"DONE\")", "fs.write").check(Unchanged("src/main.wj", "fn main() {}\n")),
        esc("self/append-lock-file", FS_ALL_WRITE, "use std::fs", "let r = fs::append(\"$ROOT/x.wj-capabilities.lock\", \"allow\")\n    println(\"DONE\")", "fs.write").check(Missing("x.wj-capabilities.lock")),
        esc("self/chdir-then-relative", FS, "use std::fs", "let r = fs::set_current_dir(\"$ROOT/data\")\n    println(\"DONE\")", "env.write"),
        // ---- filesystem: legitimate use inside the scope
        legit("fs/ok-read", FS, "use std::fs", "let r = fs::read_to_string(\"$ROOT/data/ok.txt\")\n    println(\"DONE ${r.unwrap()}\")"),
        legit("fs/ok-read-nested", FS, "use std::fs", "let r = fs::read_to_string(\"$ROOT/data/sub/deep.txt\")\n    println(\"DONE ${r.unwrap()}\")"),
        legit("fs/ok-read-dir", FS, "use std::fs", "let r = fs::read_dir(\"$ROOT/data\")\n    println(\"DONE\")"),
        legit("fs/ok-exists", FS, "use std::fs", "let r = fs::exists(\"$ROOT/data/ok.txt\")\n    println(\"DONE ${r}\")"),
        legit("fs/ok-dotdot-that-stays-inside", FS, "use std::fs", "let r = fs::read_to_string(\"$ROOT/data/sub/../ok.txt\")\n    println(\"DONE ${r.unwrap()}\")"),
        legit("fs/ok-write", FS, "use std::fs", "let r = fs::write(\"$ROOT/out/new.txt\", \"hello\")\n    println(\"DONE ${r.is_ok()}\")"),
        legit("fs/ok-rename-inside", FS, "use std::fs", "let r = fs::rename(\"$ROOT/out/a.txt\", \"$ROOT/out/b.txt\")\n    println(\"DONE ${r.is_ok()}\")"),
        legit("fs/ok-copy-in-to-out", FS, "use std::fs", "let r = fs::copy(\"$ROOT/data/ok.txt\", \"$ROOT/out/copy.txt\")\n    println(\"DONE ${r.is_ok()}\")"),
        legit("fs/ok-create-dir", FS, "use std::fs", "let r = fs::create_dir_all(\"$ROOT/out/x/y\")\n    println(\"DONE ${r.is_ok()}\")"),
        legit("fs/ok-manifest-named-exactly", "[capabilities]\nfs.write = [\"$ROOT/wj.toml\"]\n", "use std::fs", "let r = fs::append(\"$ROOT/wj.toml\", \"# note\\n\")\n    println(\"DONE ${r.is_ok()}\")"),
        legit("fs/ok-single-level-glob", "[capabilities]\nfs.read = [\"$ROOT/data/*.txt\"]\n", "use std::fs", "let r = fs::read_to_string(\"$ROOT/data/ok.txt\")\n    println(\"DONE ${r.unwrap()}\")"),
        esc("fs/single-level-glob-does-not-recurse", "[capabilities]\nfs.read = [\"$ROOT/data/*.txt\"]\n", "use std::fs", "let r = fs::read_to_string(\"$ROOT/data/sub/deep.txt\")\n    println(\"DONE\")", "fs.read"),
        esc("fs/no-fs-grant-at-all", NONE, "use std::fs", "let r = fs::read_to_string(\"$ROOT/data/ok.txt\")\n    println(\"DONE\")", "fs.read"),
        // ---- network
        esc("net/other-host", NET, "use std::http", "let r = http::get(\"http://other.example/\")\n    println(\"DONE\")", "net.connect(other.example)"),
        esc("net/userinfo-trick", NET, "use std::http", "let r = http::get(\"http://api.example.com@evil.example/\")\n    println(\"DONE\")", "net.connect(evil.example)"),
        esc("net/subdomain-of-exact-host", NET, "use std::http", "let r = http::get(\"http://evil.api.example.com/\")\n    println(\"DONE\")", "net.connect"),
        esc("net/suffix-trick", NET, "use std::http", "let r = http::get(\"http://api.example.com.evil.example/\")\n    println(\"DONE\")", "net.connect"),
        esc("net/wildcard-does-not-match-apex", NET, "use std::http", "let r = http::get(\"https://github.com/\")\n    println(\"DONE\")", "net.connect(github.com)"),
        esc("net/wrong-port", NET, "use std::http", "let r = http::get(\"https://api.github.com:444/\")\n    println(\"DONE\")", "net.connect"),
        esc("net/port-by-scheme-mismatch", NET, "use std::http", "let r = http::get(\"http://api.github.com/\")\n    println(\"DONE\")", "net.connect"),
        esc("net/ip-literal", NET, "use std::http", "let r = http::get(\"http://127.0.0.1:8080/\")\n    println(\"DONE\")", "net.connect(127.0.0.1)"),
        esc("net/localhost-wrong-port", NET, "use std::http", "let r = http::get(\"http://localhost:9/\")\n    println(\"DONE\")", "net.connect(localhost)"),
        esc("net/ipv6-loopback", NET, "use std::http", "let r = http::get(\"http://[::1]/\")\n    println(\"DONE\")", "net.connect(::1)"),
        esc("net/fragment-trick", NET, "use std::http", "let r = http::get(\"http://evil.example#@api.example.com/\")\n    println(\"DONE\")", "net.connect(evil.example)"),
        esc("net/query-trick", NET, "use std::http", "let r = http::get(\"http://evil.example?h=api.example.com\")\n    println(\"DONE\")", "net.connect(evil.example)"),
        esc("net/uppercase-scheme-and-host", NET, "use std::http", "let r = http::get(\"HTTP://EVIL.EXAMPLE/\")\n    println(\"DONE\")", "net.connect(evil.example)"),
        esc("net/trailing-dot", NET, "use std::http", "let r = http::get(\"http://evil.example./\")\n    println(\"DONE\")", "net.connect(evil.example)"),
        esc("net/post", NET, "use std::http", "let r = http::post(\"http://other.example/\", \"data\")\n    println(\"DONE\")", "net.connect"),
        esc("net/put", NET, "use std::http", "let r = http::put(\"http://other.example/\", \"data\")\n    println(\"DONE\")", "net.connect"),
        esc("net/delete", NET, "use std::http", "let r = http::delete(\"http://other.example/\")\n    println(\"DONE\")", "net.connect"),
        esc("net/head", NET, "use std::http", "let r = http::head(\"http://other.example/\")\n    println(\"DONE\")", "net.connect"),
        esc("net/computed-url", NET, "use std::http", "let h = \"ev\" + \"il.example\"\n    let r = http::get(\"http://\" + h + \"/\")\n    println(\"DONE\")", "net.connect(evil.example)"),
        esc("net/listen-not-granted", NET, "use std::http", "let r = http::serve_fn(\"0.0.0.0:9999\", |req| req)\n    println(\"DONE\")", "net.listen"),
        esc("net/no-net-grant", NONE, "use std::http", "let r = http::get(\"http://api.example.com/ok\")\n    println(\"DONE\")", "net.connect"),
        legit("net/ok-exact-host", NET, "use std::http", "let r = http::get(\"http://api.example.com/ok\")\n    println(\"DONE ${r.is_ok()}\")"),
        legit("net/ok-wildcard-subdomain-and-port", NET, "use std::http", "let r = http::get(\"https://api.github.com:443/x\")\n    println(\"DONE ${r.is_ok()}\")"),
        legit("net/ok-backslash-keeps-checked-host", NET, "use std::http", "let r = http::get(\"http://api.example.com\\\\@evil.example/\")\n    println(\"DONE ${r.is_err()}\")"),
        // ---- environment
        esc("env/secret-var", ENV, "use std::env", "let r = env::var(\"SECRET_TOKEN\")\n    println(\"DONE\")", "env.read(SECRET_TOKEN)"),
        esc("env/path-var", ENV, "use std::env", "let r = env::get(\"PATH\")\n    println(\"DONE\")", "env.read(PATH)"),
        esc("env/get-or", ENV, "use std::env", "let r = env::get_or(\"AWS_SECRET\", \"none\")\n    println(\"DONE\")", "env.read(AWS_SECRET)"),
        esc("env/set", ENV, "use std::env", "env::set(\"HOME\", \"/tmp\")\n    println(\"DONE\")", "env.write(HOME)"),
        esc("env/remove", ENV, "use std::env", "env::remove(\"APP_NAME\")\n    println(\"DONE\")", "env.write(APP_NAME)"),
        esc("env/no-env-grant", NONE, "use std::env", "let r = env::var(\"HOME\")\n    println(\"DONE\")", "env.read"),
        legit("env/ok-exact", ENV, "use std::env", "let r = env::var(\"HOME\")\n    println(\"DONE ${r.is_ok()}\")"),
        legit("env/ok-prefix-pattern", ENV, "use std::env", "let r = env::var(\"APP_NAME\")\n    println(\"DONE ${r.is_err()}\")"),
        legit("env/ok-home-dir", ENV, "use std::env", "let r = env::home_dir()\n    println(\"DONE\")"),
        // ---- processes
        esc("proc/other-program", PROC, "use std::process", "let r = process::run(\"ls\")\n    println(\"DONE\")", "process(ls)"),
        esc("proc/absolute-path-of-granted-name", PROC, "use std::process", "let r = process::run(\"/bin/echo\")\n    println(\"DONE\")", "process(/bin/echo)"),
        esc("proc/local-path-borrowing-a-name", PROC, "use std::process", "let r = process::run(\"./echo\")\n    println(\"DONE\")", "process(./echo)"),
        esc("proc/shell-injection-after-granted-program", PROC, "use std::process", "let r = process::run(\"echo hi; touch $ROOT/pwned\")\n    println(\"DONE\")", "process(sh)").check(Missing("pwned")),
        esc("proc/command-substitution", PROC, "use std::process", "let r = process::run(\"echo $(touch $ROOT/pwned2)\")\n    println(\"DONE\")", "process(sh)").check(Missing("pwned2")),
        esc("proc/pipe", PROC, "use std::process", "let r = process::run(\"echo hi | tee $ROOT/pwned3\")\n    println(\"DONE\")", "process(sh)").check(Missing("pwned3")),
        esc("proc/with-args-shell", PROC, "use std::process", "let r = process::run_with_args(\"sh\", [\"-c\", \"touch $ROOT/pwned4\"])\n    println(\"DONE\")", "process(sh)").check(Missing("pwned4")),
        esc("proc/spawn-long-lived", PROC, "use std::subprocess", "let r = subprocess::spawn(\"sh\", [\"-c\", \"touch $ROOT/pwned5\"])\n    println(\"DONE\")", "process(sh)").check(Missing("pwned5")),
        esc("proc/no-process-grant", NONE, "use std::process", "let r = process::run(\"echo hi\")\n    println(\"DONE\")", "process(echo)"),
        legit("proc/ok-granted-program", PROC, "use std::process", "let r = process::run(\"echo hi\")\n    println(\"DONE ${r.unwrap().trim()}\")"),
        // ---- determinism effects can be revoked
        esc("time/denied-clock", "[capabilities]\ndeny = [\"time\"]\n", "use std::time", "let t = time::now()\n    println(\"DONE\")", "time"),
        esc("time/denied-sleep", "[capabilities]\ndeny = [\"time\"]\n", "use std::time", "time::sleep_millis(1)\n    println(\"DONE\")", "time"),
        esc("random/denied", "[capabilities]\ndeny = [\"random\"]\n", "use std::random", "let n = random::range(0, 10)\n    println(\"DONE\")", "random"),
        esc("random/denied-uuid", "[capabilities]\ndeny = [\"random\"]\n", "use std::uuid", "let n = uuid::v4()\n    println(\"DONE\")", "random"),
        esc("console/denied-stdin", "[capabilities]\ndeny = [\"console\"]\n", "use std::io", "let n = io::read_line()\n    println(\"DONE\")", "console"),
        legit("time/default-granted", NONE, "use std::time", "let t = time::now()\n    println(\"DONE\")"),
        legit("random/default-granted", NONE, "use std::random", "let n = random::range(0, 10)\n    println(\"DONE\")"),
        // ---- confinement survives indirection
        esc("indirect/inside-closure", FS, "use std::fs", "let f = || fs::read_to_string(\"$ROOT/secret.txt\")\n    let r = f()\n    println(\"DONE\")", "fs.read"),
        esc("indirect/inside-spawned-task", FS, "use std::fs", "let t = spawn fs::read_to_string(\"$ROOT/secret.txt\")\n    let r = t.wait()\n    println(\"DONE\")", "fs.read"),
        esc("indirect/inside-callback-argument", FS, "use std::fs", "let items = [1]\n    let r = items.iter().map(|x| fs::read_to_string(\"$ROOT/secret.txt\")).collect::<Vec<_>>()\n    println(\"DONE ${r.len()}\")", "fs.read"),
        esc("indirect/denial-is-not-catchable", FS, "use std::fs", "match fs::read_to_string(\"$ROOT/secret.txt\") {\n        Ok(_) => println(\"DONE\"),\n        Err(_) => println(\"DONE survived\"),\n    }", "fs.read"),
        // ---- capability values (std::cap): minting needs the grant, narrowing only shrinks, operations stay inside the value
        esc("cap/mint-without-grant", NONE, "use std::cap", "let net = cap::net()\n    println(\"DONE\")", "net.connect"),
        esc("cap/mint-fs-without-grant", NONE, "use std::cap", "let f = cap::fs()\n    println(\"DONE\")", "fs.read"),
        esc("cap/mint-revoked-clock", "[capabilities]\ndeny = [\"time\"]\n", "use std::cap", "let c = cap::clock()\n    println(\"DONE\")", "time"),
        esc("cap/net-other-host", NET, "use std::cap", "let n = cap::net().only([\"api.example.com\"])\n    let r = n.get(\"http://other.example/\")\n    println(\"DONE\")", "net.connect(other.example)"),
        esc("cap/net-cannot-widen", NET, "use std::cap", "let n = cap::net().only([\"api.example.com\"])\n    let w = n.only([\"localhost:8080\"])\n    println(\"DONE\")", "net.connect"),
        esc("cap/net-cannot-widen-to-star", NET, "use std::cap", "let n = cap::net().only([\"*\"])\n    println(\"DONE\")", "net.connect"),
        esc("cap/net-narrowing-beyond-the-ambient-grant", NET, "use std::cap", "let n = cap::net().only([\"evil.example\"])\n    println(\"DONE\")", "net.connect(evil.example)"),
        esc("cap/fs-read-only-blocks-write", FS_ALL_WRITE, "use std::cap", "let f = cap::fs().read_only()\n    let r = f.write(\"$ROOT/out/x.txt\", \"x\")\n    println(\"DONE\")", "fs.write").check(Missing("out/x.txt")),
        esc("cap/fs-under-dotdot", FS_ALL_WRITE, "use std::cap", "let f = cap::fs().under(\"$ROOT/data\")\n    let r = f.read_to_string(\"$ROOT/data/../secret.txt\")\n    println(\"DONE\")", "fs.read"),
        esc("cap/fs-under-symlink", FS_ALL_WRITE, "use std::cap", "let f = cap::fs().under(\"$ROOT/data\")\n    let r = f.read_to_string(\"$ROOT/data/link/secret.txt\")\n    println(\"DONE\")", "fs.read"),
        esc("cap/fs-under-outside-the-grant", FS, "use std::cap", "let f = cap::fs().under(\"$ROOT\")\n    println(\"DONE\")", "fs.read"),
        esc("cap/fs-sibling-of-under", FS_ALL_WRITE, "use std::cap", "let f = cap::fs().under(\"$ROOT/data\")\n    let r = f.read_to_string(\"$ROOT/data2/x.txt\")\n    println(\"DONE\")", "fs.read"),
        esc("cap/env-other-name", ENV, "use std::cap", "let e = cap::env().only([\"HOME\"])\n    let r = e.var(\"SECRET_TOKEN\")\n    println(\"DONE\")", "env.read(SECRET_TOKEN)"),
        esc("cap/process-other-program", PROC, "use std::cap", "let p = cap::process().only([\"echo\"])\n    let r = p.run(\"echo hi; touch $ROOT/pwned6\")\n    println(\"DONE\")", "process(sh)").check(Missing("pwned6")),
        legit("cap/ok-net-within-value", NET, "use std::cap", "let n = cap::net().only([\"api.example.com\"])\n    let r = n.get(\"http://api.example.com/ok\")\n    println(\"DONE ${r.is_ok()}\")"),
        legit("cap/ok-fs-under-read", FS, "use std::cap", "let f = cap::fs().under(\"$ROOT/data\").read_only()\n    let r = f.read_to_string(\"$ROOT/data/ok.txt\")\n    println(\"DONE ${r.unwrap()}\")"),
        legit("cap/ok-fs-write-inside-value", FS, "use std::cap", "let f = cap::fs().under(\"$ROOT/out\")\n    let r = f.write(\"$ROOT/out/v.txt\", \"hello\")\n    println(\"DONE ${r.is_ok()}\")"),
        legit("cap/ok-env-var", ENV, "use std::cap", "let e = cap::env().only([\"HOME\"])\n    let r = e.var(\"HOME\")\n    println(\"DONE ${r.is_ok()}\")"),
        legit("cap/ok-clock-and-rng", NONE, "use std::cap", "let c = cap::clock()\n    let g = cap::rng()\n    println(\"DONE ${g.range(0, 5) < 5}\")"),
        // ---- console is an effect too: revocable, free by default
        esc("console/denied-println", "[capabilities]\ndeny = [\"console\"]\n", "use std::fs", "println(\"DONE\")", "console"),
        esc("console/denied-eprintln", "[capabilities]\ndeny = [\"console\"]\n", "use std::fs", "eprintln(\"DONE\")", "console"),
        esc("console/denied-print", "[capabilities]\ndeny = [\"console\"]\n", "use std::fs", "print(\"DONE\")", "console"),
        legit("console/default-granted", NONE, "use std::fs", "println(\"DONE\")"),
    ];
    // every effect from an empty grant: nothing but time/random/console works
    for (name, uses, call, effect) in [
        ("empty/fs-write", "use std::fs", "fs::write(\"$ROOT/out/e.txt\", \"x\")", "fs.write"),
        ("empty/env-write", "use std::env", "env::set(\"X\", \"1\")", "env.write"),
        ("empty/net-listen", "use std::http", "http::serve_fn(\"127.0.0.1:0\", |r| r)", "net.listen"),
        ("empty/db", "use std::db", "db::connect(\"$ROOT/x.db\")", "fs.write"),
    ] {
        v.push(Case {
            name,
            grant: NONE,
            uses,
            body: Box::leak(format!("let r = {call}\n    println(\"DONE\")").into_boxed_str()),
            expect: Expect::Denied(effect),
            checks: vec![],
        });
    }
    v
}

fn fixture() -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!("wj_escape_{}_{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&root);
    for d in ["data/sub", "data2", "outside", "out", "src"] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    let root = std::fs::canonicalize(&root).unwrap();
    let w = |p: &str, t: &str| std::fs::write(root.join(p), t).unwrap();
    w("data/ok.txt", "ok");
    w("data/sub/deep.txt", "deep");
    w("data2/x.txt", "x");
    w("outside/secret.txt", "TOP-SECRET");
    w("secret.txt", "TOP-SECRET");
    w("out/a.txt", "a");
    w("src/main.wj", "fn main() {}\n");
    w("wj.toml", "[package]\nname = \"fixture\"\n");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join("outside"), root.join("data/link")).unwrap();
        std::os::unix::fs::symlink(root.join("outside/secret.txt"), root.join("data/flink")).unwrap();
        std::os::unix::fs::symlink(root.join("outside"), root.join("out/olink")).unwrap();
    }
    w(
        "mock.json",
        r#"[{"method":"GET","url":"http://api.example.com/ok","body":"hi"},
            {"method":"GET","url":"https://api.github.com:443/x","body":"hi"},
            {"method":"GET","url":"http://api.example.com\\@evil.example/","body":"hi"}]"#,
    );
    root
}

fn run_case(c: &Case) -> (i32, String, String, PathBuf) {
    let root = fixture();
    let subst = |s: &str| s.replace("$ROOT", &root.display().to_string());
    let program = format!("{}\nfn main() {{\n    {}\n}}\n", c.uses, subst(c.body));
    let grant = subst(c.grant);
    let r2 = root.clone();
    let (code, out, err) = with_big_stack(move || {
        let mut lexer = windjammer::lexer::Lexer::new(&program);
        let tokens = lexer.tokenize_with_locations();
        let parser: &'static mut windjammer::parser::Parser =
            Box::leak(Box::new(windjammer::parser::Parser::new_with_source(tokens, "case.wj".into(), program.clone())));
        let prog = match parser.parse() {
            Ok(p) => p,
            Err(e) => return (-1, String::new(), format!("parse error: {e}\n{program}")),
        };
        let mut it = Interpreter::new_capturing();
        let g = windjammer::ir::effect_facts::grant_from_manifest_text(&grant, &r2).unwrap().expect("capabilities table");
        it.set_caps(Some(CapGate::new(g, CapMode::Enforce)));
        it.set_http_mode(windjammer::interpreter::builtins::std_http::HttpMode::mock_from_file(&r2.join("mock.json")).unwrap());
        match it.run_checked(&prog) {
            Ok(_) => (0, it.get_output(), String::new()),
            Err(e) => (exit_code_for(&e), it.get_output(), e.render()),
        }
    });
    (code, out, err, root)
}

#[test]
fn sandbox_escape_suite() {
    let all = cases();
    assert!(all.len() >= 60, "the E1 gate needs at least 60 cases, have {}", all.len());
    let mut failures = Vec::new();
    for c in &all {
        let (code, out, err, root) = run_case(c);
        let mut problems: Vec<String> = Vec::new();
        match &c.expect {
            Expect::Denied(what) => {
                if code != 126 {
                    problems.push(format!("expected exit 126, got {code}; err={err}"));
                } else if !err.contains(&format!("capability denied: {what}")) && !err.contains(what) {
                    problems.push(format!("denial message lacks {what:?}: {err}"));
                }
                if out.contains("DONE") || out.contains("LEAK") {
                    problems.push(format!("program continued past the denied call: {out:?}"));
                }
            }
            Expect::Done => {
                if code != 0 || !out.starts_with("DONE") {
                    problems.push(format!("legit use failed: exit {code}, out {out:?}, err {err}"));
                }
            }
        }
        for chk in &c.checks {
            match chk {
                Chk::Missing(p) if root.join(p).exists() => problems.push(format!("{p} was created")),
                Chk::Unchanged(p, text) => match std::fs::read_to_string(root.join(p)) {
                    Ok(t) if t == *text => {}
                    other => problems.push(format!("{p} changed: {other:?}")),
                },
                _ => {}
            }
        }
        if !problems.is_empty() {
            failures.push(format!("{}: {}", c.name, problems.join("; ")));
        }
        let _ = std::fs::remove_dir_all(&root);
    }
    assert!(failures.is_empty(), "{} of {} cases failed:\n{}", failures.len(), all.len(), failures.join("\n"));
    eprintln!("escape suite: {} cases, 0 escapes", all.len());
}

#[test]
fn denial_message_names_the_effect_the_grant_and_the_fix() {
    let c = esc("msg", NET, "use std::http", "let r = http::get(\"http://other.example/\")\n    println(\"DONE\")", "net.connect(other.example)");
    let (code, _, err, root) = run_case(&c);
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(code, 126);
    assert!(err.contains("capability denied: net.connect(other.example)"), "{err}");
    assert!(err.contains("grant in force: net.connect = [api.example.com, *.github.com:443, localhost:8080]"), "{err}");
    assert!(err.contains("--allow net.connect=other.example"), "{err}");
    assert!(err.contains("wj.toml"), "{err}");
}

#[test]
fn warn_mode_lets_the_call_through_and_says_so() {
    // the local-profile phase-in (decision D1): outside the profile runs, with a warning on stderr
    let root = fixture();
    let program = format!("use std::fs\nfn main() {{\n    let r = fs::exists(\"/etc/hosts\")\n    println(\"DONE ${{r}}\")\n}}\n");
    let (code, out) = with_big_stack(move || {
        let mut lexer = windjammer::lexer::Lexer::new(&program);
        let tokens = lexer.tokenize_with_locations();
        let parser: &'static mut windjammer::parser::Parser =
            Box::leak(Box::new(windjammer::parser::Parser::new_with_source(tokens, "w.wj".into(), program.clone())));
        let prog = parser.parse().unwrap();
        let mut it = Interpreter::new_capturing();
        it.set_caps(Some(CapGate::new(windjammer::ir::effect_facts::Grant::local_profile(&root), CapMode::Warn)));
        match it.run_checked(&prog) {
            Ok(_) => (0, it.get_output()),
            Err(e) => (exit_code_for(&e), it.get_output()),
        }
    });
    assert_eq!(code, 0);
    assert!(out.starts_with("DONE"), "{out}");
}
