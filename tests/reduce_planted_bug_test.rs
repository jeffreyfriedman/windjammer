//! `wj reduce`: planted-bug fixtures. A ~100-file generated program hides one construct that
//! trips a (fake) compiler; the reducer must shrink the program to that construct, anonymized.

#[path = "common/reduce_fixtures.rs"]
mod fx;

use std::path::Path;

/// A stand-in compiler: panics on `* 7919` anywhere in the project, otherwise succeeds.
fn fake_compiler(dir: &Path) -> std::path::PathBuf {
    let p = dir.join("fake-wj.sh");
    std::fs::write(
        &p,
        "#!/bin/sh\n# usage: fake-wj build <src> --no-cargo --no-lint -o <out>\nif grep -rqE '\\* *7919' \"$2\"; then echo \"thread 'main' panicked at planted\" >&2; exit 101; fi\nmkdir -p \"$6\"\nexit 0\n",
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

const ORIGINAL_IDENTS: [&str; 10] = [
    "Rec",
    "make_",
    "describe_",
    "planted_",
    "score",
    "weight",
    "label",
    "total",
    "m0",
    "m50",
];

fn assert_no_original_names(out: &Path, reducer_output: &std::process::Output) {
    let mut all = String::new();
    for (path, text) in fx::read_tree(out) {
        all.push_str(&path);
        all.push('\n');
        all.push_str(&text);
    }
    all.push_str(&String::from_utf8_lossy(&reducer_output.stdout));
    all.push_str(&String::from_utf8_lossy(&reducer_output.stderr));
    for id in ORIGINAL_IDENTS {
        assert!(
            !all.contains(id),
            "original identifier `{id}` leaked into output/logs:\n{all}"
        );
    }
}

#[test]
fn hundred_file_program_reduces_to_the_planted_construct() {
    let dir = tempfile::tempdir().unwrap();
    let (src, out) = (dir.path().join("src"), dir.path().join("out"));
    fx::write_program(&src, &fx::synthetic_program(100, Some(57)));
    let fake = fake_compiler(dir.path());

    let r = fx::reduce(
        &src,
        &out,
        &[
            "--oracle",
            "panic",
            "--wj-bin",
            fake.to_str().unwrap(),
            "--jobs",
            "2",
        ],
    );
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));

    let tree = fx::read_tree(&out);
    let wj_files: Vec<_> = tree.iter().filter(|(p, _)| p.ends_with(".wj")).collect();
    assert_eq!(
        wj_files.len(),
        1,
        "one file left: {:?}",
        tree.iter().map(|t| &t.0).collect::<Vec<_>>()
    );
    let text = &wj_files[0].1;
    assert!(
        text.lines().filter(|l| !l.trim().is_empty()).count() <= 8,
        "minimal:\n{text}"
    );
    assert!(text.contains("7919"), "planted construct survives:\n{text}");
    assert_no_original_names(&out, &r);

    let repro = std::fs::read_to_string(out.join("repro.md")).unwrap();
    assert!(repro.contains("| files | 101 | 1 |"), "{repro}");
    assert!(repro.contains("panic"));

    // the reduced program is still valid Windjammer
    let check = std::process::Command::new(fx::wj())
        .arg("check")
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "{}{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}

#[test]
fn cmd_oracle_reduces_a_dozen_modules_and_keeps_the_oracle_true() {
    let dir = tempfile::tempdir().unwrap();
    let (src, out) = (dir.path().join("src"), dir.path().join("out"));
    fx::write_program(&src, &fx::synthetic_program(12, Some(5)));
    let r = fx::reduce(&src, &out, &["--oracle", "cmd 'grep -rqE \"[*] *7919\" .'"]);
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    let still = std::process::Command::new("sh")
        .arg("-c")
        .arg("grep -rqE '[*] *7919' .")
        .current_dir(&out)
        .status()
        .unwrap();
    assert!(still.success(), "oracle must hold on the output");
    assert_no_original_names(&out, &r);
}

#[test]
fn oracle_that_does_not_hold_on_the_input_is_an_error_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (src, out) = (dir.path().join("src"), dir.path().join("out"));
    fx::write_program(&src, &fx::synthetic_program(3, None));
    let r = fx::reduce(&src, &out, &["--oracle", "cmd 'grep -rq 7919 .'"]);
    assert!(!r.status.success());
    assert!(String::from_utf8_lossy(&r.stderr).contains("does not hold"));
    assert!(!out.exists());
}

#[test]
fn reducer_output_never_contains_input_text_even_on_failure() {
    let dir = tempfile::tempdir().unwrap();
    let (src, out) = (dir.path().join("src"), dir.path().join("out"));
    fx::write_program(&src, &fx::synthetic_program(4, Some(2)));
    let r = fx::reduce(&src, &out, &["--oracle", "cmd 'false'", "-v"]);
    assert!(!r.status.success());
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&r.stdout),
        String::from_utf8_lossy(&r.stderr)
    );
    for id in ORIGINAL_IDENTS {
        assert!(!log.contains(id), "{id} in log: {log}");
    }
}
