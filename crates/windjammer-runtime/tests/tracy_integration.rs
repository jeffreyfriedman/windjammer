//! Integration checks for Tracy optional build.

#[cfg(feature = "tracy")]
#[test]
fn tracy_zone_creates_span_when_enabled() {
    use windjammer_runtime::profiling::tracy_zone;
    let _g = tracy_zone("integration_test");
}

#[test]
fn tracy_zone_no_op_without_feature() {
    use windjammer_runtime::profiling::tracy_zone;
    let _g = tracy_zone("noop");
}
