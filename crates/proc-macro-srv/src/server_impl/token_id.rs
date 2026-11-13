//! proc-macro server backend based on [`proc_macro_api::msg::SpanId`] as the backing span.
//! This backend is rather inflexible, used by RustRover and older rust-analyzer versions.
use std::ops::{Bound, Range};

use intern::Symbol;
use rustc_proc_macro::bridge::server;
use tt::literal_from_str;

use crate::{
    ProcMacroClientHandle,
    server_impl::bridge::{literal_into_bridge, token_tree_from_bridge, token_tree_into_bridge},
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpanId(pub u32);

impl proc_macro_api::token_stream::SpanLike for crate::SpanId {
    fn derive_ranged(&self, _: std::ops::Range<usize>) -> Self {
        *self
    }

    fn cover(self, _other: Self) -> Self {
        self
    }
}

impl std::fmt::Debug for SpanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

use SpanId as Span;

mod ours {
    use super::Span;

    pub(super) type TokenStream = crate::server_impl::bridge::ours::TokenStream<Span>;
}

mod bridge {
    use super::Span;

    pub(super) use crate::server_impl::bridge::bridge::*;

    pub(super) type TokenTree = crate::server_impl::bridge::bridge::TokenTree<Span>;
    pub(super) type Literal = crate::server_impl::bridge::bridge::Literal<Span>;
}

pub struct SpanIdServer<'a> {
    pub call_site: Span,
    pub def_site: Span,
    pub mixed_site: Span,
    pub callback: Option<ProcMacroClientHandle<'a>>,
}

impl server::Server for SpanIdServer<'_> {
    type TokenStream = ours::TokenStream;
    type Span = Span;
    type Symbol = Symbol;

    fn globals(&mut self) -> bridge::ExpnGlobals<Self::Span> {
        bridge::ExpnGlobals {
            def_site: self.def_site,
            call_site: self.call_site,
            mixed_site: self.mixed_site,
        }
    }

    fn intern_symbol(ident: &str) -> Self::Symbol {
        Symbol::intern(ident)
    }

    fn with_symbol_string(symbol: &Self::Symbol, f: impl FnOnce(&str)) {
        f(symbol.as_str())
    }

    fn track_env_var(&mut self, _: &str, _: Option<&str>) {}

    fn track_path(&mut self, _: &str) {}

    fn literal_from_str(&mut self, s: &str) -> Result<bridge::Literal, String> {
        literal_from_str(s, self.call_site)
            .map(literal_into_bridge)
            .map_err(|()| "cannot parse string into literal".to_owned())
    }

    fn emit_diagnostic(&mut self, _: bridge::Diagnostic<Self::Span>) {}

    fn ts_drop(&mut self, stream: Self::TokenStream) {
        drop(stream);
    }

    fn ts_clone(&mut self, stream: &Self::TokenStream) -> Self::TokenStream {
        stream.clone()
    }

    fn ts_is_empty(&mut self, stream: &Self::TokenStream) -> bool {
        stream.is_empty()
    }
    fn ts_from_str(&mut self, src: &str) -> Result<Self::TokenStream, String> {
        Self::TokenStream::from_str(src, self.call_site)
            .map_err(|e| format!("failed to parse str to token stream: {e}"))
    }
    fn ts_to_string(&mut self, stream: &Self::TokenStream) -> String {
        stream.to_string()
    }
    fn ts_from_token_tree(&mut self, tree: bridge::TokenTree) -> Self::TokenStream {
        Self::TokenStream::new(vec![token_tree_from_bridge(tree)])
    }

    fn ts_expand_expr(&mut self, self_: &Self::TokenStream) -> Result<Self::TokenStream, ()> {
        Ok(self_.clone())
    }

    fn ts_concat_trees(
        &mut self,
        base: Option<Self::TokenStream>,
        trees: Vec<bridge::TokenTree>,
    ) -> Self::TokenStream {
        let trees = trees.into_iter().map(token_tree_from_bridge);
        match base {
            Some(mut base) => {
                base.extend(trees);
                base
            }
            None => trees.collect(),
        }
    }

    fn ts_concat_streams(
        &mut self,
        base: Option<Self::TokenStream>,
        streams: Vec<Self::TokenStream>,
    ) -> Self::TokenStream {
        let mut streams = streams.into_iter();
        let mut stream = base.or_else(|| streams.next()).unwrap_or_default();
        stream.extend_with_streams(streams);
        stream
    }

    fn ts_into_trees(&mut self, stream: ours::TokenStream) -> Vec<bridge::TokenTree> {
        stream.iter().cloned().map(token_tree_into_bridge).collect()
    }

    fn span_debug(&mut self, span: Self::Span) -> String {
        format!("{:?}", span.0)
    }
    fn span_file(&mut self, _span: Self::Span) -> String {
        String::new()
    }
    fn span_local_file(&mut self, _span: Self::Span) -> Option<String> {
        None
    }
    fn span_save_span(&mut self, _span: Self::Span) -> usize {
        0
    }
    fn span_recover_proc_macro_span(&mut self, _id: usize) -> Self::Span {
        self.call_site
    }
    /// Recent feature, not yet in the proc_macro
    ///
    /// See PR:
    /// https://github.com/rust-lang/rust/pull/55780
    fn span_source_text(&mut self, _span: Self::Span) -> Option<String> {
        None
    }

    fn span_parent(&mut self, _span: Self::Span) -> Option<Self::Span> {
        None
    }
    fn span_source(&mut self, span: Self::Span) -> Self::Span {
        span
    }
    fn span_byte_range(&mut self, _span: Self::Span) -> Range<usize> {
        Range { start: 0, end: 0 }
    }
    fn span_join(&mut self, first: Self::Span, _second: Self::Span) -> Option<Self::Span> {
        // Just return the first span again, because some macros will unwrap the result.
        Some(first)
    }
    fn span_subspan(
        &mut self,
        span: Self::Span,
        _start: Bound<usize>,
        _end: Bound<usize>,
    ) -> Option<Self::Span> {
        // Just return the span again, because some macros will unwrap the result.
        Some(span)
    }
    fn span_resolved_at(&mut self, _span: Self::Span, _at: Self::Span) -> Self::Span {
        self.call_site
    }

    fn span_end(&mut self, _self_: Self::Span) -> Self::Span {
        self.call_site
    }

    fn span_start(&mut self, _self_: Self::Span) -> Self::Span {
        self.call_site
    }

    fn span_line(&mut self, _span: Self::Span) -> usize {
        1
    }

    fn span_column(&mut self, _span: Self::Span) -> usize {
        1
    }

    fn symbol_normalize_and_validate_ident(&mut self, string: &str) -> Result<Self::Symbol, ()> {
        // FIXME: nfc-normalize and validate idents
        Ok(<Self as server::Server>::intern_symbol(string))
    }
}
