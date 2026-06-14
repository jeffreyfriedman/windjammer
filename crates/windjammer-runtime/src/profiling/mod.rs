//! Runtime profiling helpers (Tracy, etc.).
//!
//! When the `tracy` Cargo feature is disabled (default), `tracy_zone` is a no-op with zero runtime
//! cost in release builds.

pub mod tracy;

#[cfg(feature = "tracy")]
pub mod tracy_gpu;

pub use tracy::{tracy_zone, TracyZoneGuard};

/// Type alias for Tracy zone guards (matches roadmap naming).
pub type TracyZone = TracyZoneGuard;
