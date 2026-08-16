//! Library interface for `proc-macro-srv-cli`.
//!
//! This module exposes the server main loop and protocol format for integration testing.

#![cfg(feature = "in-rust-tree")]
#![feature(rustc_private)]
#![expect(clippy::print_stdout, clippy::print_stderr)]

extern crate rustc_driver as _;

pub mod main_loop;
mod version;

use clap::{Command, ValueEnum};
use proc_macro_api::ProtocolFormat;

pub fn main() -> std::io::Result<()> {
    let v = std::env::var("RUST_ANALYZER_INTERNALS_DO_NOT_USE");
    if v.is_err() {
        eprintln!(
            "This is an IDE implementation detail, you can use this tool by exporting RUST_ANALYZER_INTERNALS_DO_NOT_USE."
        );
        eprintln!(
            "Note that this tool's API is highly unstable and may break without prior notice"
        );
        std::process::exit(122);
    }
    let matches = Command::new("proc-macro-srv")
        .args(&[
            clap::Arg::new("format")
                .long("format")
                .action(clap::ArgAction::Set)
                .default_value("json-legacy")
                .value_parser(clap::builder::EnumValueParser::<ProtocolFormatArg>::new()),
            clap::Arg::new("version")
                .long("version")
                .action(clap::ArgAction::SetTrue)
                .help("Prints the version of the proc-macro-srv"),
        ])
        .get_matches();
    if matches.get_flag("version") {
        println!("rust-analyzer-proc-macro-srv {}", version::version());
        return Ok(());
    }
    let &format = matches
        .get_one::<ProtocolFormatArg>("format")
        .expect("format value should always be present");

    let mut stdin = std::io::BufReader::new(std::io::stdin());
    let mut stdout = std::io::stdout();

    main_loop::run(&mut stdin, &mut stdout, format.into())
}

/// Wrapper for CLI argument parsing that implements `ValueEnum`.
#[derive(Copy, Clone)]
struct ProtocolFormatArg(ProtocolFormat);

impl From<ProtocolFormatArg> for ProtocolFormat {
    fn from(arg: ProtocolFormatArg) -> Self {
        arg.0
    }
}

impl ValueEnum for ProtocolFormatArg {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            ProtocolFormatArg(ProtocolFormat::JsonLegacy),
            ProtocolFormatArg(ProtocolFormat::BidirectionalPostcardPrototype),
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self.0 {
            ProtocolFormat::JsonLegacy => Some(clap::builder::PossibleValue::new("json-legacy")),
            ProtocolFormat::BidirectionalPostcardPrototype => {
                Some(clap::builder::PossibleValue::new("bidirectional-postcard-prototype"))
            }
        }
    }

    fn from_str(input: &str, _ignore_case: bool) -> Result<Self, String> {
        match input {
            "json-legacy" => Ok(ProtocolFormatArg(ProtocolFormat::JsonLegacy)),
            "bidirectional-postcard-prototype" => {
                Ok(ProtocolFormatArg(ProtocolFormat::BidirectionalPostcardPrototype))
            }
            _ => Err(format!("unknown protocol format: {input}")),
        }
    }
}
