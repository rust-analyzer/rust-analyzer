//! A standalone binary for `proc-macro-srv`.
//! Driver for proc macro server
#![cfg_attr(feature = "in-rust-tree", feature(rustc_private))]
#![cfg_attr(not(feature = "in-rust-tree"), allow(unused_crate_dependencies))]

#[cfg(feature = "in-rust-tree")]
extern crate rustc_driver as _;

fn main() -> std::io::Result<()> {
    cfg_select! {
        feature = "in-rust-tree" => proc_macro_srv_cli::main(),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "proc-macro-srv-cli needs to be compiled with the `in-rust-tree` feature to function"
                .to_owned(),
        )),
    }
}
