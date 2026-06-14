fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let stamp = format!("generated_at={}\n", chrono::Utc::now().to_rfc3339());
    std::fs::write(
        std::path::Path::new(&out_dir).join("mcp_build_stamp.txt"),
        stamp,
    )
    .ok();
}
