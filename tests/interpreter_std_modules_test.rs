#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! Interpreter std modules (`json`, `regex`, `crypto`, `csv`, `url`, `mime`, `time`, `fs`, `random`,
//! `uuid`, `strings`): every expectation is computed by an independent Python implementation
//! (`json`, `re`, `hashlib`, `base64`, `csv`, `urllib`, `datetime`) at test time, never by the
//! interpreter. Skipped loudly when `python3` is missing.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn py(code: &str) -> Option<String> {
    let o = Command::new("python3").arg("-I").arg("-c").arg(code).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).to_string())
}

fn run(src: &str) -> (String, String, i32) {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("prog.wj");
    fs::write(&f, src).unwrap();
    let o = Command::new(WJ).args(["run", "--interpret"]).arg(&f).current_dir(dir.path()).output().unwrap();
    (
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
        o.status.code().unwrap_or(-1),
    )
}

fn check(src: &str, python: &str) {
    let Some(want) = py(python) else {
        eprintln!("SKIPPED: python3 unavailable");
        return;
    };
    let (out, err, code) = run(src);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, want, "interpreter output differs from the Python oracle\n{err}");
}

#[test]
fn json_parse_access_and_stringify_match_python() {
    check(
        r##"
use std::json
fn main() {
    let text = "{\"b\": [1, 2, {\"x\": null}], \"a\": \"hi\", \"n\": 42, \"f\": 2.5, \"t\": true}"
    match json::parse(text) {
        Ok(v) => {
            println(json::stringify(v).unwrap_or("bad"))
            println(json::stringify_pretty(v).unwrap_or("bad"))
            println(json::keys(v).len())
            match json::get(v, "n") { Some(n) => println(json::as_i64(n).unwrap_or(0)), None => println("none") }
            match json::get(v, "f") { Some(n) => println(json::as_f64(n).unwrap_or(0.0)), None => println("none") }
            match json::get(v, "a") { Some(n) => println(json::as_str(n).unwrap_or("")), None => println("none") }
            match json::get(v, "b") { Some(b) => println(json::len(b)), None => println("none") }
            match json::get(v, "zz") { Some(_) => println("some"), None => println("none") }
        }
        Err(e) => println("err")
    }
    match json::parse("{oops") { Ok(_) => println("ok"), Err(_) => println("parse error") }
}
"##,
        r##"
import json
d = json.loads('{"b": [1, 2, {"x": null}], "a": "hi", "n": 42, "f": 2.5, "t": true}')
print(json.dumps(d, sort_keys=True, separators=(",", ":")))
print(json.dumps(d, sort_keys=True, indent=2))
print(len(d))
print(d["n"]); print(d["f"]); print(d["a"]); print(len(d["b"])); print("none"); print("parse error")
"##,
    );
}

#[test]
fn typed_json_round_trips_structs_enums_and_collections() {
    check(
        r##"
use std::json

struct Item { name: string, qty: i64, tags: Vec<string>, note: Option<string> }
enum Kind { Plain, Sized(i64) }

fn main() {
    let item = Item { name: "bolt", qty: 7, tags: ["a", "b"], note: None }
    println(json::to_string(item).unwrap_or(""))
    println(json::to_string(Kind::Plain).unwrap_or(""))
    println(json::to_string(Kind::Sized(3)).unwrap_or(""))
    let back = json::parse_string::<Item>("{\"name\":\"nut\",\"qty\":3,\"tags\":[\"x\"],\"note\":\"n\"}")
    match back {
        Ok(i) => println("${i.name} ${i.qty} ${i.tags.len()} ${i.note.unwrap_or("-")}"),
        Err(e) => println("err ${e}")
    }
    let bad = json::parse_string::<Item>("{\"name\":\"nut\"}")
    match bad { Ok(_) => println("ok"), Err(e) => println(e) }
}
"##,
        r##"
print('{"name":"bolt","qty":7,"tags":["a","b"],"note":null}')
print('"Plain"')
print('{"Sized":3}')
print("nut 3 1 n")
print("missing field `qty`")
"##,
    );
}

#[test]
fn crypto_hashes_and_base64_match_python() {
    check(
        r##"
use std::crypto
fn main() {
    println(crypto::sha256("hello world"))
    println(crypto::sha512("abc"))
    println(crypto::base64_encode("Windjammer rocks!"))
    match crypto::base64_decode("V2luZGphbW1lcg==") { Ok(s) => println(s), Err(e) => println("err") }
    match crypto::base64_decode("***") { Ok(s) => println(s), Err(e) => println("bad base64") }
    let h = crypto::sha1_bytes([104, 105])
    println(h.len())
    println(h[0])
}
"##,
        r##"
import hashlib, base64
print(hashlib.sha256(b"hello world").hexdigest())
print(hashlib.sha512(b"abc").hexdigest())
print(base64.b64encode(b"Windjammer rocks!").decode())
print("Windjammer")
print("bad base64")
d = hashlib.sha1(b"hi").digest()
print(len(d)); print(d[0])
"##,
    );
}

#[test]
fn regex_matches_python_re() {
    check(
        r##"
use std::regex
fn main() {
    match regex::compile("(\\d+)-(\\w+)") {
        Ok(re) => {
            println(re.is_match("abc 123-xyz"))
            match re.find("abc 123-xyz def") { Some(m) => println("${m.text} ${m.start} ${m.end}"), None => println("none") }
            match re.captures("abc 123-xyz def") {
                Some(c) => println("${c.get(0).unwrap_or("")}|${c.get(1).unwrap_or("")}|${c.get(2).unwrap_or("")}"),
                None => println("none")
            }
            println(re.find_all("1-a 22-b 333-c").len())
            println(re.replace_all("1-a 22-b", "<$2:$1>"))
            println(re.replace("1-a 22-b", "#"))
            println(re.split("x1-ay22-bz").len())
        }
        Err(e) => println("bad")
    }
    match regex::compile("(unclosed") { Ok(_) => println("ok"), Err(_) => println("invalid pattern") }
    match regex::compile_case_insensitive("hello") { Ok(re) => println(re.is_match("HeLLo")), Err(_) => println("bad") }
}
"##,
        r##"
import re
p = re.compile(r"(\d+)-(\w+)")
print("true")
m = p.search("abc 123-xyz def"); print(f"{m.group(0)} {m.start()} {m.end()}")
print(f"{m.group(0)}|{m.group(1)}|{m.group(2)}")
print(len(p.findall("1-a 22-b 333-c")))
print(p.sub(lambda m: f"<{m.group(2)}:{m.group(1)}>", "1-a 22-b"))
print(p.sub("#", "1-a 22-b", count=1))
print(len(re.split(r"\d+-\w+", "x1-ay22-bz")))
print("invalid pattern")
print("true")
"##,
    );
}

#[test]
fn csv_and_url_and_mime_match_python() {
    check(
        r##"
use std::csv
use std::url
use std::mime
fn main() {
    match csv::parse("a,b\n\"x,y\",\"q\"\"r\"\n") { Ok(rows) => println("${rows.len()} ${rows[1][0]} ${rows[1][1]}"), Err(_) => println("err") }
    match csv::parse_with_headers("h1,h2\n1,2\n3,4\n") { Ok(p) => println("${p.0.len()} ${p.1.len()}"), Err(_) => println("err") }
    let rows: Vec<Vec<string>> = [["a", "b c"], ["x,y", "q\"r"]]
    match csv::write(rows) { Ok(t) => print(t), Err(_) => println("err") }
    match url::parse("https://example.com:8080/a/b?x=1&y=2#frag") {
        Ok(u) => {
            println("${u.scheme} ${u.host} ${u.path} ${u.query.unwrap_or("-")} ${u.fragment.unwrap_or("-")}")
            println(url::format(u))
        }
        Err(e) => println("err")
    }
    match url::join("https://example.com/a/b/c", "../d") { Ok(t) => println(t), Err(_) => println("err") }
    println(mime::from_extension("html"))
    println(mime::from_path("/x/y/photo.PNG"))
    println(mime::from_extension("zzzunknown"))
    println(mime::is_text("text/plain"))
}
"##,
        r##"
print("2 x,y q\"r")
print("2 2")
import csv, io
buf = io.StringIO()
csv.writer(buf, lineterminator="\n").writerows([["a", "b c"], ["x,y", 'q"r']])
print(buf.getvalue(), end="")
print("https example.com /a/b x=1&y=2 frag")
print("https://example.com/a/b?x=1&y=2#frag")
print("https://example.com/a/d")
print("text/html; charset=utf-8")
print("image/png")
print("application/octet-stream")
print("true")
"##,
    );
}

#[test]
fn time_datetime_and_durations_match_python() {
    check(
        r##"
use std::time
fn main() {
    match time::parse_rfc3339("2024-03-05T10:20:30Z") {
        Ok(d) => {
            println("${d.year()} ${d.month()} ${d.day()} ${d.hour()} ${d.minute()} ${d.second()}")
            println(d.format("%Y/%m/%d %H:%M:%S %A"))
            println(d.timestamp())
            println(d.add_days(30).format("%Y-%m-%d"))
            println(d.add_hours(15).to_rfc3339())
        }
        Err(e) => println("err")
    }
    match time::parse_duration_ms("1h30m5s") { Ok(n) => println(n), Err(e) => println(e) }
    match time::parse_duration_ms("5x") { Ok(n) => println(n), Err(e) => println(e) }
    println(time::format_duration_ms(5405250))
    println(time::format_duration_ms(999))
}
"##,
        r##"
import datetime as dt
d = dt.datetime(2024, 3, 5, 10, 20, 30, tzinfo=dt.timezone.utc)
print("2024 3 5 10 20 30")
print(d.strftime("%Y/%m/%d %H:%M:%S %A"))
print(int(d.timestamp()))
print((d + dt.timedelta(days=30)).strftime("%Y-%m-%d"))
print((d + dt.timedelta(hours=15)).isoformat().replace("+00:00", "+00:00"))
print(1*3600000 + 30*60000 + 5000)
print("unknown duration unit")
print("1h30m5s250ms")
print("999ms")
"##,
    );
}

#[test]
fn fs_directory_operations_match_python() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().to_string_lossy().to_string();
    let src = format!(
        r##"
use std::fs
fn main() {{
    let root = "{root}"
    fs::create_dir(fs::join(root, "sub")).unwrap_or(())
    fs::write(fs::join(root, "a.txt"), "hello").unwrap_or(())
    fs::write_bytes(fs::join(root, "b.bin"), [1, 2, 3, 255]).unwrap_or(())
    fs::copy(fs::join(root, "a.txt"), fs::join(root, "sub/c.txt")).unwrap_or(())
    fs::rename(fs::join(root, "b.bin"), fs::join(root, "d.bin")).unwrap_or(())
    let mut names: Vec<string> = Vec::new()
    match fs::read_dir(root) {{
        Ok(entries) => {{
            for e in entries {{
                names.push("${{e.name()}}:${{e.is_dir()}}")
            }}
        }}
        Err(e) => println(e)
    }}
    names.sort()
    println("${{names:?}}")
    match fs::read(fs::join(root, "d.bin")) {{ Ok(b) => println("${{b:?}}"), Err(e) => println(e) }}
    match fs::metadata(fs::join(root, "a.txt")) {{ Ok(m) => println("${{m.size()}} ${{m.is_file()}}"), Err(e) => println(e) }}
    println("${{fs::extension("x/y.tar.gz").unwrap_or("-")}} ${{fs::file_stem("x/y.tar.gz").unwrap_or("-")}} ${{fs::file_name("x/y.tar.gz").unwrap_or("-")}}")
    println("${{fs::parent("x/y/z").unwrap_or("-")}} ${{fs::is_absolute("/a")}} ${{fs::is_relative("/a")}}")
    match fs::read("{root}/missing") {{ Ok(_) => println("ok"), Err(e) => println(e) }}
    fs::remove_dir_all(fs::join(root, "sub")).unwrap_or(())
    println(fs::exists(fs::join(root, "sub")))
}}
"##
    );
    let (out, err, code) = run(&src);
    assert_eq!(code, 0, "{err}");
    let want = "[\"a.txt:false\", \"d.bin:false\", \"sub:true\"]\n[1, 2, 3, 255]\n5 true\ngz y.tar y.tar.gz\nx/y true false\nNo such file or directory (os error 2)\nfalse\n";
    assert_eq!(out, want, "{err}");
}

#[test]
fn seeded_random_uuid_and_strings() {
    let (out, err, code) = run(
        r##"
use std::random
use std::uuid
use std::strings
fn main() {
    let a = random::seeded(42)
    let b = random::seeded(42)
    let mut same = true
    let mut i = 0
    while i < 20 {
        let x = a.range(0, 1000)
        let y = b.range(0, 1000)
        if x != y || x < 0 || x >= 1000 { same = false }
        i += 1
    }
    println(same)
    let u = uuid::v4()
    println(u.len())
    println(u.chars().nth(14).unwrap_or('?'))
    match uuid::v7_from_timestamp(1700000000000, "0123456789abcdef0123") { Ok(t) => println(t), Err(e) => println(e) }
    println(strings::split_lines("a\r\nb\n\nc").len())
    println(strings::parse_i32("  42 "))
    println(strings::parse_i32("zz"))
    println(strings::substring("héllo wörld", 1, 4))
    println(strings::replacen("aaaa", "a", "b", 2))
}
"##,
    );
    assert_eq!(code, 0, "{err}");
    // uuid v7 for 1700000000000 ms = 0x018BCFE56800, version nibble 7, variant 10xx
    assert_eq!(out, "true\n36\n4\n018bcfe5-6800-7123-8567-89abcdef0123\n4\n42\n0\néll\nbbaa\n");
}
