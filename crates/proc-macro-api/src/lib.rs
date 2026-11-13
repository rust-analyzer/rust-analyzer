//! Client-side Proc-Macro crate
//!
//! We separate proc-macro expanding logic to an extern program to allow
//! different implementations (e.g. wasm or dylib loading). And this crate
//! is used to provide basic infrastructure for communication between two
//! processes: Client (RA itself), Server (the external program)

#![cfg_attr(feature = "in-rust-tree", feature(rustc_private))]
#![allow(internal_features, unused_features)]

#[cfg(feature = "in-rust-tree")]
extern crate rustc_driver as _;

stdx::rustc_crates! {
    extern crate rustc_lexer or ra_ap_rustc_lexer;
}

pub mod bidirectional_protocol;
#[cfg(feature = "in-ra")]
pub mod client;
pub mod flat;
pub mod legacy_protocol;
#[cfg(feature = "in-ra")]
pub mod pool;
#[cfg(feature = "in-ra")]
pub mod process;
#[cfg(feature = "in-proc-macro-srv")]
pub mod token_stream;
pub mod transport;

use std::fmt;

/// The versions of the server protocol
pub mod version {
    pub const NO_VERSION_CHECK_VERSION: u32 = 0;
    pub const VERSION_CHECK_VERSION: u32 = 1;
    pub const ENCODE_CLOSE_SPAN_VERSION: u32 = 2;
    pub const HAS_GLOBAL_SPANS: u32 = 3;
    pub const RUST_ANALYZER_SPAN_SUPPORT: u32 = 4;
    /// Whether literals encode their kind as an additional u32 field and idents their rawness as a u32 field.
    pub const EXTENDED_LEAF_DATA: u32 = 5;
    pub const HASHED_AST_ID: u32 = 6;

    /// Current API version of the proc-macro protocol.
    pub const CURRENT_API_VERSION: u32 = HASHED_AST_ID;
}

/// Protocol format for communication between client and server.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ProtocolFormat {
    /// JSON-based legacy protocol (newline-delimited JSON).
    JsonLegacy,
    /// Bidirectional postcard protocol with sub-request support.
    BidirectionalPostcardPrototype,
}

impl fmt::Display for ProtocolFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolFormat::JsonLegacy => write!(f, "json-legacy"),
            ProtocolFormat::BidirectionalPostcardPrototype => {
                write!(f, "bidirectional-postcard-prototype")
            }
        }
    }
}

/// Represents different kinds of procedural macros that can be expanded by the external server.
#[derive(Copy, Clone, Eq, PartialEq, Debug, serde_derive::Serialize, serde_derive::Deserialize)]
pub enum ProcMacroKind {
    /// A macro that derives implementations for a struct or enum.
    CustomDerive,
    /// An attribute-like procedural macro.
    Attr,
    // This used to be called FuncLike, so that's what the server expects currently.
    #[serde(alias = "Bang")]
    #[serde(rename(serialize = "FuncLike", deserialize = "FuncLike"))]
    Bang,
}
