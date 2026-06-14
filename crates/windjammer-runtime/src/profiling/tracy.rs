//! CPU zones for [Tracy](https://github.com/wolfpld/tracy).
//!
//! Windjammer maps `@profile("Name")` on functions to:
//! `let _guard = windjammer_runtime::profiling::tracy_zone("Name");` at the start of the body.

#[cfg(feature = "tracy")]
use tracy_client::Client;

/// Opaque guard that ends the Tracy zone when dropped.
///
/// When the `tracy` feature is off, this is a zero-sized no-op.
pub struct TracyZoneGuard {
    #[cfg(feature = "tracy")]
    _span: tracy_client::Span,
    #[cfg(not(feature = "tracy"))]
    _no_copy: std::marker::PhantomData<()>,
}

/// Begin a named Tracy zone; drops at end of scope.
///
/// With `--features tracy` on `windjammer-runtime`, this forwards to
/// [`tracy_client::Client::span_alloc`]. Without the feature, this compiles to an empty body and
/// returns a zero-sized guard (no linked Tracy code).
#[inline]
pub fn tracy_zone(_name: &'static str) -> TracyZoneGuard {
    #[cfg(feature = "tracy")]
    {
        let client = Client::start();
        let span = client.span_alloc(Some(_name), "windjammer", file!(), line!(), 0);
        TracyZoneGuard { _span: span }
    }
    #[cfg(not(feature = "tracy"))]
    {
        TracyZoneGuard {
            _no_copy: std::marker::PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracy_zone_no_panic_without_feature() {
        {
            let _z = tracy_zone("unit_test_zone");
        }
    }
}
