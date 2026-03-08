// Marker Types Module
//
// Provides zero-cost marker types for compile-time type tracking.
// These types have no runtime representation but enable powerful
// type-level programming patterns.

use std::marker::PhantomData as StdPhantomData;

/// Zero-sized type used to mark things that "act like" they own a T.
///
/// PhantomData<T> is used to tell the compiler that a type uses T,
/// even though T never appears in the struct's fields.
///
/// Common use cases:
/// - Generic wrapper types that don't directly store T
/// - Type-safe handles/IDs that track what they point to
/// - Zero-cost type-level state machines
///
/// # Examples
///
/// ```
/// use windjammer_runtime::marker::PhantomData;
///
/// // Type-safe buffer handle
/// pub struct BufferId<T> {
///     id: u32,
///     _phantom: PhantomData<T>,
/// }
///
/// // Compiler knows this BufferId is for CameraUniforms
/// let camera_buffer: BufferId<CameraUniforms> = BufferId {
///     id: 42,
///     _phantom: PhantomData,
/// };
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct PhantomData<T: ?Sized>(StdPhantomData<T>);

impl<T: ?Sized> PhantomData<T> {
    /// Creates a new PhantomData.
    pub const fn new() -> PhantomData<T> {
        PhantomData(StdPhantomData)
    }
}

// Re-export for convenient use
pub use self::PhantomData as Phantom;
