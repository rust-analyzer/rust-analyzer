//! Bidirectional protocol methods

pub mod msg;
#[cfg(feature = "in-ra")]
mod sender;

#[cfg(feature = "in-ra")]
pub use self::sender::*;
