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

//! Regen-unblock contract for multipass component libraries (dogfood).
//!
//! Host skip flags exist because regenerated components historically dropped
//! `*_runtime_js`, required forbidden `.clone()` (W0005), or failed multipass
//! compose / string-concat borrow consistency.
//!
//! Broader full-tree regen contract: `codegen_component_library_regen_gates_test`.

#[path = "common/test_utils.rs"]
mod test_utils;

/// Minimal repro behind CurrencyInput HTML builders: `a + b + c` where `a`/`b`/`c`
/// are owned `String` temps must emit consistent `&` for `Add<&str>`.
#[test]
fn owned_string_plus_owned_string_chain_must_rustc() {
    let source = r#"
pub fn build_html(name: string, extra: string) -> string {
    let open = "<div>".to_string()
    let mid = name
    let close = "</div>".to_string()
    open + mid + extra + close
}

fn main() {
    let name = "x".to_string()
    let extra = " y".to_string()
    println!("{}", build_html(name, extra))
}
"#;
    let generated = test_utils::compile_single(source);
    // Concat body is the regen footgun; call-site literal demotion is covered elsewhere.
    assert!(
        generated.contains("open + &mid")
            || generated.contains("open + mid")
            || generated.contains("open+") ,
        "expected string concat codegen. Got:\n{generated}"
    );
    let check = test_utils::verify_rust_compiles(&generated);
    assert!(
        check.is_ok(),
        "owned String + owned String chain must rustc (regen HTML builders). stderr={:?}\nGot:\n{generated}",
        check.err()
    );
}

/// Field reuse across if/else must auto-clone — WJ source must not need `.clone()` (W0005).
#[test]
fn currency_input_field_reuse_without_explicit_clone_must_rustc() {
    let source = r#"
pub trait Renderable {
    fn render(self) -> string
}

fn format_cents(cents: int) -> string {
    if cents == 0 {
        return "0.00"
    }
    "1.00"
}

pub struct CurrencyInput {
    name: string,
    input_id: string,
    label: string,
    extra_attrs: string,
    value_cents: int,
    required: bool,
}

impl CurrencyInput {
    pub fn new() -> CurrencyInput {
        CurrencyInput {
            name: "amount".to_string(),
            input_id: "".to_string(),
            label: "".to_string(),
            extra_attrs: "".to_string(),
            value_cents: 0,
            required: false,
        }
    }

    pub fn input_id(self, input_id: string) -> CurrencyInput {
        self.input_id = input_id
        self
    }

    pub fn label(self, label: string) -> CurrencyInput {
        self.label = label
        self
    }

    pub fn extra_attrs(self, extra_attrs: string) -> CurrencyInput {
        self.extra_attrs = extra_attrs
        self
    }
}

impl Renderable for CurrencyInput {
    fn render(self) -> string {
        let value = format_cents(self.value_cents)
        let id = if self.input_id.len() == 0 {
            self.name
        } else {
            self.input_id
        }
        let req = if self.required { " required" } else { "" }
        let extra = if self.extra_attrs.len() == 0 {
            "".to_string()
        } else {
            " ".to_string() + self.extra_attrs
        }
        let label_html = if self.label.len() == 0 {
            "".to_string()
        } else {
            "<label for=\"".to_string() + id + "\">" + self.label + "</label>"
        }
        "<div class=\"wj-currency-input\">".to_string()
            + label_html
            + "<input id=\""
            + id
            + "\" name=\""
            + self.name
            + "\" value=\""
            + value
            + "\""
            + extra
            + req
            + " /></div>"
    }
}

fn main() {
    let html = CurrencyInput::new()
        .input_id("checkAmount".to_string())
        .label("Amount".to_string())
        .extra_attrs("data-wj-write-check-amount".to_string())
        .render()
    println!("{}", html)
}
"#;

    let generated = test_utils::compile_single(source);
    assert!(
        !generated.contains("error[E") && !generated.contains("error:"),
        "codegen must succeed without explicit .clone(). Got:\n{generated}"
    );
    let check = test_utils::verify_rust_compiles(&generated);
    assert!(
        check.is_ok(),
        "CurrencyInput field reuse without .clone() must rustc (also covers String+String concat borrow consistency). stderr={:?}\nGot:\n{generated}",
        check.err()
    );
    assert!(
        generated.contains("wj-currency-input") && generated.contains("checkAmount"),
        "must retain dogfood attrs. Got:\n{generated}"
    );
}

#[test]
fn auth_fetch_static_str_runtime_helper_must_codegen() {
    let source = r####"
pub fn auth_fetch_runtime_js() -> &'static str {
    r##"
(function () {
  if (window.__wjAuthFetchBound) return;
  window.__wjAuthFetchBound = true;
  window.lkSetSyncStatus = function (state) {};
  window.wjAuthFetch = async function (btn) {};
})();
"##
}

fn main() {
    let _ = auth_fetch_runtime_js();
}
"####;

    let result = test_utils::compile_single(source);
    assert!(
        result.contains("pub fn auth_fetch_runtime_js")
            && result.contains("&'static str")
            && result.contains("lkSetSyncStatus")
            && result.contains("wjAuthFetch")
            && !result.contains("error[E")
            && !result.contains("error:"),
        "AuthFetch runtime_js &'static str must codegen. Got:\n{result}"
    );
    let check = test_utils::verify_rust_compiles(&result);
    assert!(
        check.is_ok(),
        "auth_fetch_runtime_js must rustc. stderr={:?}\nGot:\n{result}",
        check.err()
    );
}

#[test]
fn write_check_form_static_str_runtime_helper_must_codegen() {
    let source = r####"
pub fn write_check_form_runtime_js() -> &'static str {
    r##"
(function () {
  if (window.__wjWriteCheckBound) return;
  window.__wjWriteCheckBound = true;
})();
"##
}

fn main() {
    let _ = write_check_form_runtime_js();
}
"####;

    let result = test_utils::compile_single(source);
    assert!(
        result.contains("pub fn write_check_form_runtime_js")
            && result.contains("&'static str")
            && result.contains("__wjWriteCheckBound")
            && !result.contains("error[E")
            && !result.contains("error:"),
        "WriteCheckForm runtime_js &'static str must codegen. Got:\n{result}"
    );
    let check = test_utils::verify_rust_compiles(&result);
    assert!(
        check.is_ok(),
        "write_check_form_runtime_js must rustc. stderr={:?}\nGot:\n{result}",
        check.err()
    );
}

/// Library multipass: WriteCheckForm composes CurrencyInput + keeps runtime_js; cargo-check.
#[test]
fn write_check_form_composing_currency_input_library_must_rustc() {
    use std::fs;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();

    fs::write(
        src.join("traits.wj"),
        r#"
pub trait Renderable {
    fn render(self) -> string
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("currencyinput.wj"),
        r#"
use super::traits::Renderable

pub struct CurrencyInput {
    name: string,
    input_id: string,
    label: string,
    extra_attrs: string,
}

impl CurrencyInput {
    pub fn new() -> CurrencyInput {
        CurrencyInput {
            name: "amount".to_string(),
            input_id: "".to_string(),
            label: "".to_string(),
            extra_attrs: "".to_string(),
        }
    }
    pub fn name(self, name: string) -> CurrencyInput { self.name = name; self }
    pub fn input_id(self, input_id: string) -> CurrencyInput { self.input_id = input_id; self }
    pub fn label(self, label: string) -> CurrencyInput { self.label = label; self }
    pub fn extra_attrs(self, extra_attrs: string) -> CurrencyInput { self.extra_attrs = extra_attrs; self }
}

impl Renderable for CurrencyInput {
    fn render(self) -> string {
        let id = if self.input_id.len() == 0 { self.name } else { self.input_id }
        "<div class=\"wj-currency-input\"><input id=\"".to_string()
            + id
            + "\" name=\""
            + self.name
            + "\" "
            + self.extra_attrs
            + "/></div>"
    }
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("writecheckform.wj"),
        r####"
use super::traits::Renderable
use super::currencyinput::CurrencyInput

pub struct WriteCheckForm {
    bank_code: string,
}

impl WriteCheckForm {
    pub fn new() -> WriteCheckForm {
        WriteCheckForm { bank_code: "1000".to_string() }
    }
}

impl Renderable for WriteCheckForm {
    fn render(self) -> string {
        let amount = CurrencyInput::new()
            .name("amount".to_string())
            .input_id("checkAmount".to_string())
            .label("Amount".to_string())
            .extra_attrs("data-wj-write-check-amount".to_string())
            .render()
        "<div class=\"wj-write-check-form\" data-wj-write-check data-wj-bank-code=\"".to_string()
            + self.bank_code
            + "\">"
            + amount
            + "</div>"
    }
}

pub fn write_check_form_runtime_js() -> &'static str {
    r##"(function(){ if (window.__wjWriteCheckBound) return; window.__wjWriteCheckBound = true; })();"##
}
"####,
    )
    .unwrap();

    fs::write(
        src.join("mod.wj"),
        "pub mod traits\npub mod currencyinput\npub mod writecheckform\n",
    )
    .unwrap();

    let out = tmp.path().join("out");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--module-file",
            "--library",
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("wj");

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&status.stdout),
        String::from_utf8_lossy(&status.stderr)
    );
    assert!(
        status.status.success(),
        "wj library build must succeed (no W0005 hard-fail on auto-clone path). out:\n{combined}"
    );

    let wc = fs::read_to_string(out.join("writecheckform.rs")).unwrap_or_default();
    let ci = fs::read_to_string(out.join("currencyinput.rs")).unwrap_or_default();

    assert!(
        ci.contains("wj-currency-input") && wc.contains("CurrencyInput::new"),
        "compose must call CurrencyInput. currencyinput:\n{ci}\nwritecheckform:\n{wc}"
    );
    assert!(
        wc.contains("write_check_form_runtime_js") && wc.contains("__wjWriteCheckBound"),
        "runtime_js must survive library codegen. Got:\n{wc}"
    );

    fs::write(
        out.join("Cargo.toml"),
        r#"[package]
name = "wjui_regen_probe"
version = "0.1.0"
edition = "2021"
[workspace]
[lib]
path = "lib.rs"
"#,
    )
    .unwrap();
    if !out.join("lib.rs").exists() {
        fs::write(
            out.join("lib.rs"),
            "pub mod traits;\npub mod currencyinput;\npub mod writecheckform;\n",
        )
        .unwrap();
    }

    let cargo = std::process::Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&out)
        .output()
        .expect("cargo check");
    assert!(
        cargo.status.success(),
        "regen library must cargo-check.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&cargo.stdout),
        String::from_utf8_lossy(&cargo.stderr)
    );
}
