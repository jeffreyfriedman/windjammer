#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn wdb_lsn_is_at_or_before_no_borrow_through() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "wal_layout.wj",
        r#"
pub struct Lsn {
    pub value: u64,
}

impl Lsn {
    pub fn is_at_or_before(self, other: Lsn) -> bool {
        self.value <= other.value
    }
}

pub struct WalRecord {
    pub lsn: Lsn,
}

pub fn decode_records(bytes: Vec<u8>) -> Vec<WalRecord> {
    let _ = bytes.len()
    Vec::new()
}

pub fn replay_to_lsn(path: string, through: Lsn) -> Vec<WalRecord> {
    let all = decode_records(Vec::new())
    let mut out = Vec::new()
    for record in all {
        if record.lsn.is_at_or_before(through) {
            out.push(record)
        }
    }
    let _ = path
    out
}
"#,
    );
    test.add_file("main.wj", "pub fn run() {}");
    let map = test.compile().expect("compile");
    let rs = map.get("wal_layout.rs").expect("wal_layout.rs");
    assert!(
        !rs.contains("is_at_or_before(&through)"),
        "Copy Lsn must pass by value into owned formal. Got:\n{rs}"
    );
}
