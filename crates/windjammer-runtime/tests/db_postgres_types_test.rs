//! `std::db` against a real PostgreSQL: every integer width, NUMERIC and the other common column types
//! read back as the value the server holds. Skipped (with a message) when the PostgreSQL server
//! binaries are not installed.
#![cfg(feature = "db")]

use std::path::PathBuf;
use std::process::{Command, Stdio};

fn pg_bin(name: &str) -> Option<PathBuf> {
    for dir in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/lib/postgresql/16/bin", "/usr/lib/postgresql/15/bin", "/usr/bin"] {
        let p = PathBuf::from(dir).join(name);
        if p.exists() {
            return Some(p);
        }
    }
    Command::new("which").arg(name).output().ok().filter(|o| o.status.success()).map(|o| {
        PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string())
    })
}

struct Cluster {
    dir: tempfile::TempDir,
    pg_ctl: PathBuf,
    url: String,
}

impl Drop for Cluster {
    fn drop(&mut self) {
        let _ = Command::new(&self.pg_ctl).args(["-D"]).arg(self.dir.path().join("data")).args(["-m", "immediate", "stop"]).output();
    }
}

fn start_cluster() -> Option<Cluster> {
    let initdb = pg_bin("initdb")?;
    let pg_ctl = pg_bin("pg_ctl")?;
    let dir = tempfile::tempdir().ok()?;
    let data = dir.path().join("data");
    let ok = Command::new(&initdb)
        .args(["-D"])
        .arg(&data)
        .args(["-U", "wj", "--auth=trust", "-E", "UTF8"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?
        .success();
    if !ok {
        return None;
    }
    let port = std::net::TcpListener::bind("127.0.0.1:0").ok()?.local_addr().ok()?.port();
    let sock = dir.path().to_string_lossy().to_string();
    let started = Command::new(&pg_ctl)
        .args(["-D"])
        .arg(&data)
        .args(["-w", "-o", &format!("-p {port} -k {sock} -c listen_addresses=127.0.0.1"), "start"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?
        .success();
    if !started {
        return None;
    }
    Some(Cluster { dir, pg_ctl, url: format!("postgres://wj@127.0.0.1:{port}/postgres") })
}

#[test]
fn postgres_column_types_read_back_exactly() {
    let Some(cluster) = start_cluster() else {
        eprintln!("skipped: PostgreSQL server binaries (initdb, pg_ctl) not available");
        return;
    };
    let conn = windjammer_runtime::db::connect(&cluster.url).expect("connect");
    let rows = conn
        .query(
            "SELECT 7::int4 AS a, 7::int2 AS b, 7::int8 AS c, (SELECT SUM(x) FROM (SELECT 7::bigint AS x) s)::numeric AS d, \
             12.50::numeric AS e, (-0.05)::numeric AS f, true AS g, 1.5::float4 AS h, (-0.25)::float8 AS i, \
             'hi'::text AS j, '550e8400-e29b-41d4-a716-446655440000'::uuid AS k, '\\xdead'::bytea AS l, \
             '2024-02-29'::date AS m, '2024-02-29 13:05:09.25'::timestamp AS n, \
             '2024-02-29 13:05:09+00'::timestamptz AS o, '{\"a\": 1}'::jsonb AS p, '{\"a\":1}'::json AS q, \
             ARRAY[1,2,3]::int4[] AS r, ARRAY['a','b c']::text[] AS s, NULL::int4 AS t, 1234567.89::numeric AS u, \
             (SELECT 5::int4) AS v, COUNT(*)::int4 AS w FROM generate_series(1,3)",
            Vec::new(),
        )
        .expect("query");
    let row = &rows[0];
    let int = |c: &str| row.get_int(c).unwrap_or_else(|e| panic!("{c}: {e}"));
    let text = |c: &str| row.get_string(c).unwrap_or_else(|e| panic!("{c}: {e}"));
    assert_eq!((int("a"), int("b"), int("c"), int("d")), (7, 7, 7, 7));
    assert_eq!(text("e"), "12.50");
    assert_eq!(text("f"), "-0.05");
    assert_eq!(text("g"), "true");
    assert_eq!(text("h"), "1.5");
    assert_eq!(text("i"), "-0.25");
    assert_eq!(text("j"), "hi");
    assert_eq!(text("k"), "550e8400-e29b-41d4-a716-446655440000");
    assert_eq!(text("l"), "\\xdead");
    assert_eq!(text("m"), "2024-02-29");
    assert_eq!(text("n"), "2024-02-29 13:05:09.25");
    assert_eq!(text("o"), "2024-02-29 13:05:09+00");
    assert_eq!(text("p"), "{\"a\": 1}");
    assert_eq!(text("q"), "{\"a\":1}");
    assert_eq!(text("r"), "{1,2,3}");
    assert_eq!(text("s"), "{a,\"b c\"}");
    assert_eq!(text("t"), "");
    assert_eq!(text("u"), "1234567.89");
    assert_eq!((int("v"), int("w")), (5, 3));
}
