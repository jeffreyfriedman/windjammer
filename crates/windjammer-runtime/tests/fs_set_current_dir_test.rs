//! `fs::set_current_dir` changes the process-wide working directory, so it is tested in its own test process.

use windjammer_runtime::fs;

#[test]
fn set_current_dir_changes_the_working_directory_and_reports_errors() {
    let before = std::env::current_dir().unwrap();
    let tmp = std::env::temp_dir();
    fs::set_current_dir(&tmp).unwrap();
    assert_eq!(std::env::current_dir().unwrap().canonicalize().unwrap(), tmp.canonicalize().unwrap());
    fs::set_current_dir(&before).unwrap();
    assert!(fs::set_current_dir(tmp.join("windjammer-no-such-dir")).is_err());
}
