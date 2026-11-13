//! The initial proc-macro-srv protocol, soon to be deprecated.

pub mod msg;
#[cfg(feature = "in-ra")]
mod sender;

#[cfg(feature = "in-ra")]
pub(crate) use self::sender::*;

/// Legacy span type, only defined here as it is still used by the proc-macro server.
/// While rust-analyzer doesn't use this anymore at all, RustRover relies on the legacy type for
/// proc-macro expansion.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpanId(pub u32);

impl std::fmt::Debug for SpanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
