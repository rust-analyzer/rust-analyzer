//! limit defines a struct to enforce limits.

/// Represents a struct used to enforce a numerical limit.
pub struct Limit {
    upperbound: usize,
}

impl Limit {
    /// Creates a new limit.
    #[inline]
    pub const fn new(upperbound: usize) -> Self {
        Self { upperbound }
    }

    /// Gets the underlying numeric limit.
    #[inline]
    pub const fn inner(&self) -> usize {
        self.upperbound
    }

    /// Checks whether the given value is below the limit.
    /// Returns `Ok` when `other` is below `self`, and `Err` otherwise.
    #[inline]
    pub const fn check(&self, other: usize) -> Result<(), ()> {
        if other > self.upperbound {
            Err(())
        } else {
            Ok(())
        }
    }
}
