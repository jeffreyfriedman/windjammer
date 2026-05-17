//! GPU zones for Tracy (timestamp pairs resolved on the CPU).
//!
//! This module re-exports [`tracy_client`] GPU types. Wiring `wgpu` timestamp queries to
//! [`GpuSpan::upload_timestamp_start`] / [`GpuSpan::upload_timestamp_end`] is engine-specific;
//! use a `wgpu::QuerySet` with `QUERY_TYPE_TIMESTAMP` (or backend-specific timestamp queries), map
//! the result buffer, then pass nanosecond values to [`GpuSpan`].

pub use tracy_client::{Client, GpuContext, GpuContextCreationError, GpuContextType, GpuSpan, GpuSpanCreationError};
